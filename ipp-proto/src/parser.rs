//!
//! IPP stream parser
//!
use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};
use log::{debug, error};
use num_traits::FromPrimitive;

use crate::{ipp::*, *};

#[derive(Debug)]
pub enum ParseError {
    InvalidTag(u8),
    InvalidVersion,
    InvalidCollection,
    IOError(io::Error),
}

impl From<io::Error> for ParseError {
    fn from(error: io::Error) -> Self {
        ParseError::IOError(error)
    }
}

// create a single value from one-element list, list otherwise
fn list_or_value(mut list: Vec<IppValue>) -> IppValue {
    if list.len() == 1 {
        list.remove(0)
    } else {
        IppValue::ListOf(list)
    }
}

/// IPP parsing result
pub struct IppParseResult {
    pub header: IppHeader,
    pub attributes: IppAttributes,
}

impl IppParseResult {
    fn new(header: IppHeader, attributes: IppAttributes) -> IppParseResult {
        IppParseResult { header, attributes }
    }
}

/// IPP parser implementation
pub struct IppParser<'a> {
    reader: &'a mut Read,
    last_delimiter: DelimiterTag,
    last_name: Option<String>,
    context: Vec<Vec<IppValue>>,
    attributes: IppAttributes,
}

impl<'a> IppParser<'a> {
    /// Create IPP parser using the given Read
    pub fn new(reader: &'a mut Read) -> IppParser<'a> {
        IppParser {
            reader,
            last_delimiter: DelimiterTag::EndOfAttributes,
            last_name: None,
            context: vec![vec![]],
            attributes: IppAttributes::new(),
        }
    }

    fn add_last_attribute(&mut self) {
        if let Some(ref last_name) = self.last_name {
            if let Some(val_list) = self.context.pop() {
                self.attributes.add(
                    self.last_delimiter,
                    IppAttribute::new(&last_name, list_or_value(val_list)),
                );
            }
            self.context.push(vec![]);
        }
    }

    fn parse_delimiter(&mut self, tag: u8) -> Result<bool, ParseError> {
        debug!("Delimiter tag: {:0x}", tag);
        if tag == DelimiterTag::EndOfAttributes as u8 {
            // end of stream, add last attribute
            self.add_last_attribute();
            Ok(true)
        } else {
            // remember delimiter tag
            self.last_delimiter = DelimiterTag::from_u8(tag).ok_or_else(|| ParseError::InvalidTag(tag))?;
            Ok(false)
        }
    }

    fn parse_value(&mut self, tag: u8) -> Result<(), ParseError> {
        // value tag
        let namelen = self.reader.read_u16::<BigEndian>()?;
        let name = self.reader.read_string(namelen as usize)?;
        let value = IppValue::read(tag, &mut self.reader)?;

        debug!("Value tag: {:0x}: {}: {}", tag, name, value);

        if namelen > 0 {
            // single attribute or begin of array
            self.add_last_attribute();
            // store it as a previous attribute
            self.last_name = Some(name);
        }
        if tag == ValueTag::BegCollection as u8 {
            // start new collection in the stack
            debug!("Begin collection");
            match value {
                IppValue::Other { tag: _, ref data } if data.is_empty() => {}
                _ => {
                    error!("Invalid begin collection attribute");
                    return Err(ParseError::InvalidCollection);
                }
            }
            self.context.push(vec![]);
        } else if tag == ValueTag::EndCollection as u8 {
            // get collection from the stack and add it to the previous element
            debug!("End collection");
            match value {
                IppValue::Other { tag: _, ref data } if data.is_empty() => {}
                _ => {
                    error!("Invalid end collection attribute");
                    return Err(ParseError::InvalidCollection);
                }
            }
            if let Some(arr) = self.context.pop() {
                if let Some(val_list) = self.context.last_mut() {
                    val_list.push(IppValue::Collection(arr));
                }
            }
        } else if let Some(val_list) = self.context.last_mut() {
            // add attribute to the current collection
            val_list.push(value);
        }
        Ok(())
    }

    /// Parse IPP stream
    pub fn parse(mut self) -> Result<IppParseResult, ParseError> {
        let header = IppHeader::from_reader(self.reader)?;
        debug!("IPP header: {:?}", header);

        loop {
            match self.reader.read_u8()? {
                tag @ 0x01...0x05 => {
                    if self.parse_delimiter(tag)? {
                        break;
                    }
                }
                tag @ 0x10...0x4a => self.parse_value(tag)?,
                tag => {
                    return Err(ParseError::InvalidTag(tag));
                }
            }
        }

        Ok(IppParseResult::new(header, self.attributes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_no_attributes() {
        let data = &[1, 1, 0, 0, 0, 0, 0, 0, 3];
        let result = IppParser::new(&mut Cursor::new(data)).parse();
        assert!(result.is_ok());

        let res = result.ok().unwrap();
        assert!(res.attributes.job_attributes().is_none());
        assert!(res.attributes.printer_attributes().is_none());
        assert!(res.attributes.operation_attributes().is_none());
    }

    #[test]
    fn test_parse_single_value() {
        let data = &[
            1, 1, 0, 0, 0, 0, 0, 0, 4, 0x21, 0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x04, 0x12, 0x34, 0x56, 0x78, 3,
        ];
        let result = IppParser::new(&mut Cursor::new(data)).parse();
        assert!(result.is_ok());

        let res = result.ok().unwrap();
        let attrs = res.attributes.printer_attributes().unwrap();
        let attr = attrs.get("test").unwrap();
        if let IppValue::Integer(val) = attr.value() {
            assert_eq!(*val, 0x12345678);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_parse_list() {
        let data = &[
            1, 1, 0, 0, 0, 0, 0, 0, 4, 0x21, 0x00, 0x04, b't', b'e', b's', b't', 0x00, 0x04, 0x12, 0x34, 0x56, 0x78,
            0x21, 0x00, 0x00, 0x00, 0x04, 0x77, 0x65, 0x43, 0x21, 3,
        ];
        let result = IppParser::new(&mut Cursor::new(data)).parse();
        assert!(result.is_ok());

        let res = result.ok().unwrap();
        let attrs = res.attributes.printer_attributes().unwrap();
        let attr = attrs.get("test").unwrap();
        if let IppValue::ListOf(list) = attr.value() {
            assert_eq!(
                *list,
                vec![IppValue::Integer(0x12345678), IppValue::Integer(0x77654321)]
            );
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_parse_collection() {
        let data = vec![
            1, 1, 0, 0, 0, 0, 0, 0, 4, 0x34, 0, 4, b'c', b'o', b'l', b'l', 0, 0, 0x21, 0, 0, 0, 4, 0x12, 0x34, 0x56,
            0x78, 0x44, 0, 0, 0, 3, b'k', b'e', b'y', 0x37, 0, 0, 0, 0, 3,
        ];
        let result = IppParser::new(&mut Cursor::new(data)).parse();
        assert!(result.is_ok());

        let res = result.ok().unwrap();
        let attrs = res.attributes.printer_attributes().unwrap();
        let attr = attrs.get("coll").unwrap();
        if let IppValue::Collection(coll) = attr.value() {
            assert_eq!(
                *coll,
                vec![IppValue::Integer(0x12345678), IppValue::Keyword("key".to_owned())]
            );
        } else {
            assert!(false);
        }
    }
}