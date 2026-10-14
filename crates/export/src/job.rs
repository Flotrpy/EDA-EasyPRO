//! Gerber job file (`.gbrjob`, Ucamco spec): one JSON file that tells the
//! fab what every layer file is, plus board size, thickness and rules.

use easypro_core::geom::Rect;
use easypro_core::pcb::Pcb;
use serde_json::{json, Value};

fn mm(nm: i64) -> f64 {
    (nm as f64 / 1e3).round() / 1e3
}

/// `files` pairs each file name with its X2 file function and polarity.
pub fn job_file(pcb: &Pcb, project: &str, files: &[(String, String, String)]) -> String {
    let size = pcb.outline().map(Rect::from_points).unwrap_or_default();
    let rules = &pcb.rules;
    let v: Value = json!({
        "Header": {
            "GenerationSoftware": { "Vendor": "EasyPRO", "Application": "EasyPRO", "Version": env!("CARGO_PKG_VERSION") },
        },
        "GeneralSpecs": {
            "ProjectId": { "Name": project, "Revision": "1" },
            "Size": { "X": mm(size.width()), "Y": mm(size.height()) },
            "LayerNumber": pcb.copper_layers,
            "BoardThickness": mm(pcb.thickness),
            "Finish": "Other"
        },
        "DesignRules": [{
            "Layers": "Outer",
            "PadToPad": mm(rules.clearance),
            "PadToTrack": mm(rules.clearance),
            "TrackToTrack": mm(rules.clearance),
            "MinLineWidth": mm(rules.min_track_width),
        }],
        "FilesAttributes": files.iter().map(|(path, function, polarity)| json!({
            "Path": path,
            "FileFunction": function,
            "FilePolarity": polarity,
        })).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&v).expect("job serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_is_valid_json() {
        let p = easypro_core::examples::blinker();
        let files = vec![(
            "b.GTL".to_string(),
            "Copper,L1,Top".to_string(),
            "Positive".to_string(),
        )];
        let text = job_file(&p.boards[0].pcb, "blinker", &files);
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["GeneralSpecs"]["Size"]["X"], 50.0);
        assert_eq!(v["GeneralSpecs"]["BoardThickness"], 1.6);
        assert_eq!(v["FilesAttributes"][0]["Path"], "b.GTL");
    }
}
