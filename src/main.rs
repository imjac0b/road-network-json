use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct GeoJsonFeature {
    #[serde(rename = "type")]
    feature_type: String,
    geometry: Geometry,
    properties: HashMap<String, PropertyValue>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Geometry {
    #[serde(rename = "type")]
    geometry_type: String,
    coordinates: Vec<Vec<f64>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum PropertyValue {
    String(String),
    Int(i64),
    Float(f64),
    Null,
}

fn main() -> Result<()> {
    let data_dir = "./input";
    let output_dir = "./output";
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;
    fs::create_dir_all(format!("{}/centerlines", output_dir))
        .context("Failed to create centerlines directory")?;
    fs::create_dir_all(format!("{}/pedestrian_zones", output_dir))
        .context("Failed to create pedestrian_zones directory")?;

    println!("Parsing GML files...");

    // Parse centerlines
    let cl_path = format!("{}/CENTERLINE.gml", data_dir);
    if Path::new(&cl_path).exists() {
        println!("Processing CENTERLINE.gml...");
        parse_gml_file(&cl_path, "ROUTE_ID", "centerlines", output_dir)?;
    } else {
        println!("Warning: {} not found", cl_path);
    }

    // Parse pedestrian zones
    let pz_path = format!("{}/PEDESTRIAN_ZONE.gml", data_dir);
    if Path::new(&pz_path).exists() {
        println!("Processing PEDESTRIAN_ZONE.gml...");
        parse_gml_file(&pz_path, "PED_ZONE_ID", "pedestrian_zones", output_dir)?;
    } else {
        println!("Warning: {} not found", pz_path);
    }

    println!("Done! JSON files have been written to {}/", output_dir);
    Ok(())
}

fn parse_gml_file(
    file_path: &str,
    id_field: &str,
    output_subdir: &str,
    output_dir: &str,
) -> Result<()> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path))?;

    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut in_city_object = false;
    let mut current_object = String::new();
    let mut object_depth = 0;
    let mut count = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name.ends_with(":GenericCityObject") {
                    in_city_object = true;
                    object_depth = 1;
                    current_object.clear();
                    current_object.push_str(&format!("<{}", name));
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            current_object.push_str(&format!(
                                " {}=\"{}\"",
                                String::from_utf8_lossy(attr.key.as_ref()),
                                String::from_utf8_lossy(&attr.value)
                            ));
                        }
                    }
                    current_object.push('>');
                } else if in_city_object {
                    object_depth += 1;
                    current_object.push_str(&format!("<{}", name));
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            current_object.push_str(&format!(
                                " {}=\"{}\"",
                                String::from_utf8_lossy(attr.key.as_ref()),
                                String::from_utf8_lossy(&attr.value)
                            ));
                        }
                    }
                    current_object.push('>');
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if in_city_object {
                    current_object.push_str(&format!("</{}>", name));
                    object_depth -= 1;
                    if object_depth == 0 {
                        // Process the complete city object
                        if let Ok(feature) = parse_city_object(&current_object, id_field) {
                            if let Some(id) = feature.properties.get(id_field) {
                                let id_str = match id {
                                    PropertyValue::String(s) => s.clone(),
                                    PropertyValue::Int(i) => i.to_string(),
                                    _ => format!("object_{}", count),
                                };
                                
                                let output_path = format!("{}/{}/{}.json", output_dir, output_subdir, id_str);
                                let json = serde_json::to_string_pretty(&feature)?;
                                let mut file = File::create(&output_path)?;
                                file.write_all(json.as_bytes())?;
                                count += 1;
                                
                                if count % 100 == 0 {
                                    println!("  Processed {} features...", count);
                                }
                            }
                        }
                        in_city_object = false;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_city_object {
                    let text = e.unescape().unwrap_or_default();
                    current_object.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("Error at position {}: {:?}", reader.buffer_position(), e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    println!("  Total features processed: {}", count);
    Ok(())
}

fn parse_city_object(xml: &str, _id_field: &str) -> Result<GeoJsonFeature> {
    let mut properties = HashMap::new();
    let mut coordinates = Vec::new();

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut in_string_attr = false;
    let mut in_int_attr = false;
    let mut in_double_attr = false;
    let mut in_pos_list = false;
    let mut current_attr_name = String::new();
    let mut current_value = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                
                if name.ends_with(":stringAttribute") {
                    in_string_attr = true;
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            if key == "name" {
                                current_attr_name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                } else if name.ends_with(":intAttribute") {
                    in_int_attr = true;
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            if key == "name" {
                                current_attr_name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                } else if name.ends_with(":doubleAttribute") {
                    in_double_attr = true;
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            if key == "name" {
                                current_attr_name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                } else if name.ends_with(":posList") {
                    in_pos_list = true;
                    current_value.clear();
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                
                if name.ends_with(":stringAttribute") {
                    properties.insert(current_attr_name.clone(), PropertyValue::String(current_value.clone()));
                    in_string_attr = false;
                    current_value.clear();
                } else if name.ends_with(":intAttribute") {
                    if let Ok(val) = current_value.parse::<i64>() {
                        properties.insert(current_attr_name.clone(), PropertyValue::Int(val));
                    }
                    in_int_attr = false;
                    current_value.clear();
                } else if name.ends_with(":doubleAttribute") {
                    if let Ok(val) = current_value.parse::<f64>() {
                        properties.insert(current_attr_name.clone(), PropertyValue::Float(val));
                    }
                    in_double_attr = false;
                    current_value.clear();
                } else if name.ends_with(":posList") {
                    // Parse coordinates from posList
                    let coords: Vec<f64> = current_value
                        .split_whitespace()
                        .filter_map(|s| s.parse::<f64>().ok())
                        .collect();
                    
                    // Convert HK80 to WGS84
                    let from_proj = "+proj=tmerc +lat_0=22.31213333333334 +lon_0=114.1785555555556 +k=1 +x_0=836694.05 +y_0=819069.8 +ellps=intl +towgs84=-162.619,-276.959,-161.764,0.067753,-2.24365,-1.15883,-1.09425 +units=m +no_defs";
                    let to_proj = "+proj=longlat +datum=WGS84 +no_defs";
                    
                    if let (Ok(from), Ok(to)) = (
                        proj4rs::Proj::from_proj_string(from_proj),
                        proj4rs::Proj::from_proj_string(to_proj),
                    ) {
                        for chunk in coords.chunks(2) {
                            if chunk.len() == 2 {
                                let mut point = (chunk[0], chunk[1], 0.0);
                                // Transform from HK80 to WGS84
                                if proj4rs::transform::transform(&from, &to, &mut point).is_ok() {
                                    // point now contains (longitude, latitude, z)
                                    coordinates.push(vec![point.0, point.1]);
                                }
                            }
                        }
                    }
                    
                    in_pos_list = false;
                    current_value.clear();
                } else if name.ends_with(":value") {
                    // Value is already collected in current_value
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default();
                if in_string_attr || in_int_attr || in_double_attr || in_pos_list {
                    current_value.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(GeoJsonFeature {
        feature_type: "Feature".to_string(),
        geometry: Geometry {
            geometry_type: "LineString".to_string(),
            coordinates,
        },
        properties,
    })
}
