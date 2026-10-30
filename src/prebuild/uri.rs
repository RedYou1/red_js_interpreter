use crate::prebuild::prelude::*;

const UNRESERVED: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()";
const URI_RESERVED: &str = ";/?:@&=+$,#[]";

pub fn prebuild_uri(environment: Environment) {
    let function = Prototype::find(environment.mem.clone(), &stringify!(Function).into())
        .1
        .borrow()
        .unwrap_proto("URI functions for Function");
    for (name, encoder, preserve_reserved) in [
        ("encodeURI", true, true),
        ("encodeURIComponent", true, false),
        ("decodeURI", false, true),
        ("decodeURIComponent", false, false),
    ] {
        let runnable = prebuild_runnable(
            environment.clone(),
            Box::new(move |_, _, [input]| {
                let input = match inline_borrow!(input) {
                    JsValue::String(value) => value,
                    value => value.print(),
                };
                let result = if encoder {
                    encode_uri(&input, preserve_reserved)
                } else {
                    decode_uri(&input, preserve_reserved)
                };
                match result {
                    Ok(result) => {
                        CodeResult::Return(Rc::new(RefCell::new(JsValue::String(result))))
                    }
                    Err(error) => CodeResult::Error(Rc::new(RefCell::new(JsValue::String(error)))),
                }
            }),
        );
        environment
            .mem
            .borrow_mut()
            .properties
            .insert(name.into(), new_runnable(function.clone(), name, runnable));
    }
}

fn encode_uri(input: &str, preserve_reserved: bool) -> Result<String, String> {
    let mut encoded = String::new();
    for byte in input.as_bytes() {
        let character = *byte as char;
        if character.is_ascii()
            && (UNRESERVED.contains(character)
                || preserve_reserved && URI_RESERVED.contains(character))
        {
            encoded.push(character);
        } else {
            use std::fmt::Write;
            write!(&mut encoded, "%{byte:02X}").map_err(|error| error.to_string())?;
        }
    }
    Ok(encoded)
}

fn decode_uri(input: &str, preserve_reserved: bool) -> Result<String, String> {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        if index + 2 >= bytes.len() {
            return Err("malformed URI escape".to_owned());
        }
        let high = hex_digit(bytes[index + 1]).ok_or_else(|| "malformed URI escape".to_owned())?;
        let low = hex_digit(bytes[index + 2]).ok_or_else(|| "malformed URI escape".to_owned())?;
        let decoded_byte = high * 16 + low;
        if preserve_reserved && URI_RESERVED.contains(decoded_byte as char) {
            decoded.extend_from_slice(&bytes[index..index + 3]);
        } else {
            decoded.push(decoded_byte);
        }
        index += 3;
    }
    String::from_utf8(decoded).map_err(|error| error.to_string())
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
