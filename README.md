# Road Network GML to JSON Parser

A Rust tool to parse Hong Kong Transport Department's "Road Network (2nd Generation)" GML files and convert them into individual GeoJSON files.

## Overview

This tool parses GML files containing road centerlines and pedestrian zones, transforms coordinates from HK80 (EPSG:2326) to WGS84 (EPSG:4326), and outputs individual JSON files for each feature.

## Features

- **GML Parsing**: Reads CENTERLINE.gml and PEDESTRIAN_ZONE.gml files
- **Coordinate Transformation**: Converts HK80 coordinates to WGS84 (GeoJSON-compatible)
- **Individual JSON Files**: Creates one JSON file per feature for easy lookup by ID
- **GeoJSON Format**: Output follows GeoJSON specification with LineString geometries
- **Type-Safe Properties**: Preserves string, integer, and double attributes from the GML

## Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- GML files from Hong Kong Transport Department:
  - `CENTERLINE.gml`
  - `PEDESTRIAN_ZONE.gml`

## Installation

1. Clone or download this repository
2. Place your GML files in a `gml/` directory in the project root

## Usage

```bash
# Build and run
cargo run --release

# Or build first, then run
cargo build --release
./target/release/road-network-json
```

## Input Structure

Place your GML files in the following structure:

```
road-network-json/
├── gml/
│   ├── CENTERLINE.gml
│   └── PEDESTRIAN_ZONE.gml
└── ...
```

## Output Structure

The tool creates the following output structure:

```
output/
├── centerlines/
│   ├── <ROUTE_ID>.json
│   ├── <ROUTE_ID>.json
│   └── ...
└── pedestrian_zones/
    ├── <PED_ZONE_ID>.json
    ├── <PED_ZONE_ID>.json
    └── ...
```

## Output Format

Each JSON file contains a GeoJSON Feature:

```json
{
  "type": "Feature",
  "geometry": {
    "type": "LineString",
    "coordinates": [
      [114.17855, 22.31213],
      [114.17856, 22.31214],
      ...
    ]
  },
  "properties": {
    "ROUTE_ID": "123",
    "STREET_NAME_EN": "Example Road",
    "DISTRICT": "Central",
    ...
  }
}
```

## Dependencies

- `quick-xml`: Fast XML parsing
- `serde` & `serde_json`: JSON serialization
- `proj4rs`: Pure Rust coordinate transformation (no system dependencies)
- `anyhow`: Error handling

## Performance

The parser processes features in batches and prints progress every 100 features. Large GML files (hundreds of MB) are handled efficiently through streaming parsing.

## License

This tool is provided as-is for working with Hong Kong Transport Department open data.
