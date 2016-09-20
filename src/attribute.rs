//!
//! Attribute-related structs
//!
use std::collections::BTreeMap;
use std::io::Write;
use byteorder::{BigEndian, WriteBytesExt};

use ::{Result, IppError};
use value::IppValue;
use consts::tag::*;
use consts::attribute::*;

const HEADER_ATTRS: [&'static str; 3] = [
    ATTRIBUTES_CHARSET,
    ATTRIBUTES_NATURAL_LANGUAGE,
    PRINTER_URI];

fn is_header_attr(attr: &str) -> bool {
    HEADER_ATTRS.into_iter().find(|&&at| at == attr).is_some()
}

/// IppAttribute represents an IPP attribute
#[derive(Clone, Debug)]
pub struct IppAttribute {
    /// Attribute name
    name: String,
    /// Attribute value
    value: IppValue
}

impl IppAttribute {
    /// Create new instance of the attribute
    /// * name - Attribute name
    /// * value - Attribute value
    pub fn new(name: &str, value: IppValue) -> IppAttribute {
        IppAttribute {name: name.to_string(), value: value}
    }

    /// Return attribute name
    pub fn name<'a>(&'a self) -> &str {
        &self.name
    }

    /// Return attribute value
    pub fn value<'a>(&'a self) -> &IppValue {
        &self.value
    }

    /// Serialize attribute into binary stream
    pub fn write(&self, writer: &mut Write) -> Result<usize> {
        let mut retval = 0;

        try!(writer.write_u8(self.value.to_tag()));
        retval += 1;

        try!(writer.write_u16::<BigEndian>(self.name.len() as u16));
        retval += 2;

        try!(writer.write_all(self.name.as_bytes()));
        retval += self.name.len();

        retval += try!(self.value.write(writer));

        Ok(retval)
    }
}

/// Attribute list indexed by group and name
#[derive(Clone)]
pub struct IppAttributeList {
    attributes: BTreeMap<u8, BTreeMap<String, IppAttribute>>
}

impl IppAttributeList {
    /// Create attribute list
    pub fn new() -> IppAttributeList {
        IppAttributeList { attributes: BTreeMap::new() }
    }

    /// Add attribute to the list
    /// * group - delimiter group
    /// * attribute - attribute to add
    pub fn add(&mut self, group: u8, attribute: IppAttribute) {
        if !self.attributes.contains_key(&group) {
            self.attributes.insert(group, BTreeMap::new());
        }
        let mut opt = self.attributes.get_mut(&group).unwrap();
        opt.insert(attribute.name().to_string(), attribute);
    }

    /// Get attribute from the list
    pub fn get<'a>(&'a self, group: u8, name: &str) -> Option<&IppAttribute> {
        match self.attributes.get(&group) {
            Some(attrs) => attrs.get(name),
            None => None
        }
    }

    /// Get attribute list for a group
    pub fn get_group<'a>(&'a self, group: u8) -> Option<&BTreeMap<String, IppAttribute>> {
        self.attributes.get(&group)
    }

    /// Serialize attribute list into binary stream
    pub fn write(&self, writer: &mut Write) -> Result<usize> {
        // first send the header attributes
        try!(writer.write_u8(OPERATION_ATTRIBUTES_TAG));

        let mut retval = 1;

        for hdr in HEADER_ATTRS.into_iter() {
            match self.get(OPERATION_ATTRIBUTES_TAG, hdr) {
                Some(attr) => retval += try!(attr.write(writer)),
                None => {
                    error!("Missing required operation attribute: {}", hdr);
                    return Err(IppError::RequestError);
                }
            }
        }

        // now the rest
        for hdr in [OPERATION_ATTRIBUTES_TAG, JOB_ATTRIBUTES_TAG, PRINTER_ATTRIBUTES_TAG].into_iter() {
            let group = *hdr;
            match self.get_group(group) {
                Some(attrs) => {
                    if group != OPERATION_ATTRIBUTES_TAG {
                        try!(writer.write_u8(group));
                        retval += 1;
                    }
                    for (_, attr) in attrs.iter().filter(
                        |&(_, v)| group != OPERATION_ATTRIBUTES_TAG || !is_header_attr(v.name())) {
                        retval += try!(attr.write(writer));
                    }
                },
                None => {}
            }
        }
        try!(writer.write_u8(END_OF_ATTRIBUTES_TAG));
        retval += 1;

        Ok(retval)
    }
}