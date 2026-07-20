use palmerge_core::{message, ErrorCode, Locale, MessageKey, PalError};
use palmerge_parser::{inspect_all, Inspection};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Json,
}

struct Options {
    path: PathBuf,
    locale: Locale,
    output: OutputFormat,
}

fn main() -> ExitCode {
    let raw: Vec<String> = env::args().skip(1).collect();
    if raw
        .first()
        .is_some_and(|argument| argument == "--version" || argument == "-V")
    {
        println!("{}", version());
        return ExitCode::SUCCESS;
    }
    let locale = requested_locale(&raw).unwrap_or_default();
    match parse_options(&raw).and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error_json(&error));
            eprintln!("{}", message(locale, MessageKey::Usage));
            ExitCode::from(2)
        }
    }
}

fn version() -> String {
    format!("palmerge {}", env!("CARGO_PKG_VERSION"))
}

fn requested_locale(args: &[String]) -> Option<Locale> {
    args.windows(2)
        .find(|pair| pair[0] == "--lang")
        .and_then(|pair| Locale::parse(&pair[1]).ok())
}

fn parse_options(args: &[String]) -> Result<Options, PalError> {
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!(
            "{}",
            message(
                requested_locale(args).unwrap_or_default(),
                MessageKey::Usage
            )
        );
        std::process::exit(0);
    }
    if args.first().map(String::as_str) != Some("inspect") {
        return Err(PalError::new(
            ErrorCode::InvalidArguments,
            "expected inspect command",
        ));
    }
    let path = args
        .get(1)
        .filter(|arg| !arg.starts_with('-'))
        .ok_or_else(|| PalError::new(ErrorCode::InvalidArguments, "missing save path"))?;
    let mut locale = Locale::English;
    let mut output = OutputFormat::Text;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--lang" => {
                index += 1;
                locale = Locale::parse(args.get(index).ok_or_else(|| {
                    PalError::new(ErrorCode::InvalidArguments, "missing language value")
                })?)?;
            }
            "--format" => {
                index += 1;
                output = match args.get(index).map(String::as_str) {
                    Some("text") => OutputFormat::Text,
                    Some("json") => OutputFormat::Json,
                    _ => {
                        return Err(PalError::new(
                            ErrorCode::InvalidArguments,
                            "format must be text or json",
                        ))
                    }
                };
            }
            value => {
                return Err(PalError::new(
                    ErrorCode::InvalidArguments,
                    format!("unknown argument: {value}"),
                ))
            }
        }
        index += 1;
    }
    Ok(Options {
        path: PathBuf::from(path),
        locale,
        output,
    })
}

fn run(options: Options) -> Result<(), PalError> {
    let inspections = inspect_all(&options.path)?;
    match options.output {
        OutputFormat::Text => print_text(options.locale, &inspections),
        OutputFormat::Json => println!("{}", inspections_json(&inspections)),
    }
    Ok(())
}

fn print_text(locale: Locale, inspections: &[Inspection]) {
    println!("{}", message(locale, MessageKey::Inspecting));
    for inspection in inspections {
        println!();
        println!(
            "{}: {}",
            message(locale, MessageKey::File),
            inspection.fingerprint.path.display()
        );
        println!(
            "{}: {}",
            message(locale, MessageKey::Size),
            inspection.fingerprint.size
        );
        println!(
            "{}: {}",
            message(locale, MessageKey::Sha256),
            inspection.fingerprint.sha256
        );
        println!(
            "{}: {}",
            message(locale, MessageKey::Modified),
            inspection
                .fingerprint
                .modified_unix_seconds
                .map_or_else(|| "null".to_owned(), |value| value.to_string())
        );
        println!(
            "{}: {}",
            message(locale, MessageKey::Format),
            inspection.format.as_str()
        );
        if let Some(container) = inspection.container {
            println!(
                "{}: {}",
                message(locale, MessageKey::Container),
                container.kind.as_str()
            );
            println!(
                "{}: {}",
                message(locale, MessageKey::Compression),
                container.compression.as_str()
            );
        }
        if let Some(decoded) = inspection.decoded {
            println!(
                "{}: {}",
                message(locale, MessageKey::DecodedSize),
                decoded.decoded_len
            );
            println!(
                "{}: {}",
                message(locale, MessageKey::EmbeddedFormat),
                if decoded.embedded_gvas {
                    "gvas"
                } else {
                    "unknown"
                }
            );
        }
        if let Some(gvas) = &inspection.gvas {
            println!(
                "{}: {}",
                message(locale, MessageKey::SaveGameVersion),
                gvas.save_game_version
            );
            println!(
                "{}: {} / {}",
                message(locale, MessageKey::PackageVersion),
                gvas.package_version.ue4,
                gvas.package_version
                    .ue5
                    .map_or_else(|| "null".to_owned(), |value| value.to_string())
            );
            println!(
                "{}: {}.{}.{} ({})",
                message(locale, MessageKey::EngineVersion),
                gvas.engine_version.major,
                gvas.engine_version.minor,
                gvas.engine_version.patch,
                gvas.engine_version.build
            );
            println!(
                "{}: {}",
                message(locale, MessageKey::EngineBranch),
                gvas.engine_version.branch
            );
            println!(
                "{}: {}",
                message(locale, MessageKey::CustomVersions),
                gvas.custom_versions.len()
            );
            println!(
                "{}: {}",
                message(locale, MessageKey::SaveGameClass),
                gvas.save_game_class
            );
        }
        if let Some(properties) = &inspection.properties {
            println!(
                "{}: {}",
                message(locale, MessageKey::TopLevelProperties),
                properties.properties.len()
            );
        }
    }
}

fn inspections_json(inspections: &[Inspection]) -> String {
    let entries = inspections
        .iter()
        .map(inspection_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"schema_version\":1,\"operation\":\"inspect\",\"read_only\":true,\"files\":[{entries}]}}")
}

fn inspection_json(value: &Inspection) -> String {
    let modified = value
        .fingerprint
        .modified_unix_seconds
        .map_or_else(|| "null".to_owned(), |time| time.to_string());
    let container = value.container.map_or_else(
        || "null".to_owned(),
        |header| {
            format!(
                "{{\"kind\":\"{}\",\"compression\":\"{}\",\"save_type\":{},\"uncompressed_len\":{},\"compressed_len\":{},\"payload_offset\":{},\"chunk_wrapped\":{}}}",
                header.kind.as_str(),
                header.compression.as_str(),
                header.save_type,
                header.uncompressed_len,
                header.compressed_len,
                header.payload_offset,
                header.chunk_wrapped
            )
        },
    );
    let decoded = value.decoded.map_or_else(
        || "null".to_owned(),
        |summary| {
            format!(
                "{{\"decoded_len\":{},\"embedded_gvas\":{}}}",
                summary.decoded_len, summary.embedded_gvas
            )
        },
    );
    let gvas = value.gvas.as_ref().map_or_else(
        || "null".to_owned(),
        |header| {
            let ue5 = header
                .package_version
                .ue5
                .map_or_else(|| "null".to_owned(), |version| version.to_string());
            let custom_format = header
                .custom_format_version
                .map_or_else(|| "null".to_owned(), |version| version.to_string());
            let custom_versions = header
                .custom_versions
                .iter()
                .map(|version| {
                    format!(
                        "{{\"guid\":\"{}\",\"value\":{}}}",
                        json_escape(&version.guid),
                        version.value
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"save_game_version\":{},\"package_version\":{{\"ue4\":{},\"ue5\":{}}},\"engine_version\":{{\"major\":{},\"minor\":{},\"patch\":{},\"build\":{},\"branch\":\"{}\"}},\"custom_format_version\":{},\"custom_versions\":[{}],\"save_game_class\":\"{}\"}}",
                header.save_game_version,
                header.package_version.ue4,
                ue5,
                header.engine_version.major,
                header.engine_version.minor,
                header.engine_version.patch,
                header.engine_version.build,
                json_escape(&header.engine_version.branch),
                custom_format,
                custom_versions,
                json_escape(&header.save_game_class)
            )
        },
    );
    let properties = value.properties.as_ref().map_or_else(
        || "null".to_owned(),
        |inventory| {
            let entries = inventory
                .properties
                .iter()
                .map(|property| {
                    let metadata = &property.metadata;
                    format!(
                        "{{\"name\":\"{}\",\"type\":\"{}\",\"size\":{},\"array_index\":{},\"property_guid\":{},\"metadata\":{{\"bool_value\":{},\"enum_type\":{},\"inner_type\":{},\"key_type\":{},\"value_type\":{},\"struct_type\":{},\"struct_guid\":{}}}}}",
                        json_escape(&property.name),
                        json_escape(&property.property_type),
                        property.size,
                        property.array_index,
                        optional_json_string(property.property_guid.as_deref()),
                        metadata
                            .bool_value
                            .map_or_else(|| "null".to_owned(), |value| value.to_string()),
                        optional_json_string(metadata.enum_type.as_deref()),
                        optional_json_string(metadata.inner_type.as_deref()),
                        optional_json_string(metadata.key_type.as_deref()),
                        optional_json_string(metadata.value_type.as_deref()),
                        optional_json_string(metadata.struct_type.as_deref()),
                        optional_json_string(metadata.struct_guid.as_deref())
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"count\":{},\"entries\":[{}]}}",
                inventory.properties.len(),
                entries
            )
        },
    );
    format!(
        "{{\"path\":\"{}\",\"size\":{},\"modified_unix_seconds\":{},\"sha256\":\"{}\",\"format\":\"{}\",\"container\":{},\"decoded\":{},\"gvas\":{},\"properties\":{}}}",
        json_escape(&value.fingerprint.path.to_string_lossy()), value.fingerprint.size, modified,
        value.fingerprint.sha256, value.format.as_str(), container, decoded, gvas, properties
    )
}

fn error_json(error: &PalError) -> String {
    format!(
        "{{\"schema_version\":1,\"code\":\"{}\",\"message\":\"{}\"}}",
        error.code.as_str(),
        json_escape(&error.message)
    )
}

fn optional_json_string(value: Option<&str>) -> String {
    value.map_or_else(
        || "null".to_owned(),
        |text| format!("\\\"{}\\\"", json_escape(text)),
    )
}

fn json_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if c <= '\u{1f}' => output.push_str(&format!("\\u{:04x}", c as u32)),
            c => output.push(c),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_json_control_characters() {
        assert_eq!(json_escape("a\n\"b\\"), "a\\n\\\"b\\\\");
    }

    #[test]
    fn parses_chinese_json_options() {
        let args = [
            "inspect",
            "Level.sav",
            "--lang",
            "zh-CN",
            "--format",
            "json",
        ]
        .map(str::to_owned);
        let options = parse_options(&args).unwrap();
        assert_eq!(options.locale, Locale::SimplifiedChinese);
        assert_eq!(options.output, OutputFormat::Json);
    }

    #[test]
    fn version_is_available_without_runtime_dependencies() {
        assert_eq!(version(), format!("palmerge {}", env!("CARGO_PKG_VERSION")));
    }
}
