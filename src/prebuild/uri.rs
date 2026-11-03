use crate::prebuild::prelude::*;
#[derive(Clone, Copy)]
enum UriMode {
    EncodeUri,
    EncodeComponent,
    DecodeUri,
    DecodeComponent,
}

fn encode_uri(value: &str, component: bool) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::new();
    for byte in value.bytes() {
        let unescaped = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')')
            || (!component
                && matches!(
                    byte,
                    b';' | b',' | b'/' | b'?' | b':' | b'@' | b'&' | b'=' | b'+' | b'$' | b'#'
                ));
        if unescaped {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    encoded
}

fn hex_byte(bytes: &[u8], index: usize) -> Option<u8> {
    let high = (*bytes.get(index + 1)? as char).to_digit(16)?;
    let low = (*bytes.get(index + 2)? as char).to_digit(16)?;
    Some((high * 16 + low) as u8)
}

fn is_reserved(byte: u8) -> bool {
    matches!(
        byte,
        b';' | b'/' | b'?' | b':' | b'@' | b'&' | b'=' | b'+' | b'$' | b',' | b'#'
    )
}

fn decode_uri(value: &str, preserve_reserved: bool) -> Result<String, ()> {
    let bytes = value.as_bytes();
    let mut decoded = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            let character = value[index..].chars().next().ok_or(())?;
            decoded.push(character);
            index += character.len_utf8();
            continue;
        }

        let first = hex_byte(bytes, index).ok_or(())?;
        if preserve_reserved && is_reserved(first) {
            decoded.push_str(&value[index..index + 3]);
            index += 3;
            continue;
        }

        let mut encoded = Vec::new();
        while index < bytes.len() && bytes[index] == b'%' {
            let byte = hex_byte(bytes, index).ok_or(())?;
            if preserve_reserved && is_reserved(byte) {
                break;
            }
            encoded.push(byte);
            index += 3;
        }
        decoded.push_str(&String::from_utf8(encoded).map_err(|_| ())?);
    }
    Ok(decoded)
}

fn uri_error(env: Environment) -> CodeResult {
    let error = Prototype::find(env.mem, &stringify!(URIError).into())
        .1
        .borrow()
        .unwrap_proto("URIError");
    CodeResult::Error(Rc::new(RefCell::new(JsValue::Prototype(
        Prototype::new_child(error, None, []),
    ))))
}

fn install_uri_function(env: Environment, name: &'static str, mode: UriMode) {
    let function = Prototype::find(env.mem.clone(), &stringify!(Function).into())
        .1
        .borrow()
        .unwrap_proto("URI function");
    let runnable = prebuild_runnable_direct(
        env.clone(),
        Box::new(move |env, _, arguments| {
            let value = arguments
                .first()
                .map(|value| inline_borrow!(value.clone()))
                .unwrap_or(JsValue::Undefined);
            let value = value.print();
            let result = match mode {
                UriMode::EncodeUri => Ok(encode_uri(&value, false)),
                UriMode::EncodeComponent => Ok(encode_uri(&value, true)),
                UriMode::DecodeUri => decode_uri(&value, true),
                UriMode::DecodeComponent => decode_uri(&value, false),
            };
            match result {
                Ok(value) => CodeResult::Return(Rc::new(RefCell::new(JsValue::String(value)))),
                Err(()) => uri_error(env),
            }
        }),
    );
    env.mem
        .borrow_mut()
        .properties
        .insert(name.into(), new_builtin_runnable(function, name, runnable));
}

pub fn prebuild_uri(env: Environment) {
    install_uri_function(env.clone(), "encodeURI", UriMode::EncodeUri);
    install_uri_function(env.clone(), "encodeURIComponent", UriMode::EncodeComponent);
    install_uri_function(env.clone(), "decodeURI", UriMode::DecodeUri);
    install_uri_function(env, "decodeURIComponent", UriMode::DecodeComponent);
}
