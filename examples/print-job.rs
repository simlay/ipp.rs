extern crate ipp;
extern crate env_logger;

use std::env;
use std::process::exit;
use std::fs::File;

use ipp::attribute::IppAttribute;
use ipp::value::IppValue;
use ipp::operation::{PrintJob, IppOperation};
use ipp::consts::tag::JOB_ATTRIBUTES_TAG;

pub fn main() {
    env_logger::init().unwrap();

    let args: Vec<_> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: {} uri filename [key=value ...]", args[0]);
        exit(1);
    }

    let mut f = File::open(&args[2]).unwrap();

    let mut operation = PrintJob::new(
        &args[1],
        &mut f, &env::var("USER").unwrap(), Some(&args[1])
    );

    for arg in &args[3..] {
        let mut kv = arg.split("=");
        let (k, v) = (kv.next().unwrap(), kv.next().unwrap());

        let value = if let Ok(iv) = v.parse::<i32>() {
            IppValue::Integer(iv)
        } else if v == "true" || v == "false" {
            IppValue::Boolean(v == "true")
        } else {
            IppValue::Keyword(v.to_string())
        };

        operation.set_job_attribute(IppAttribute::new(k, value));
    }

    let attrs = operation.execute().unwrap();

    for (_, v) in attrs.get_group(JOB_ATTRIBUTES_TAG).unwrap() {
        println!("{}: {}", v.name(), v.value());
    }
}