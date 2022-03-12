use std::fs::read_to_string;

use crate::verification::lvs::LvsInput;

use super::{create_run_file, NetgenLvsOpts};

#[test]
fn test_create_run_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let netlist = temp_dir.path().join("netlist.spice");
    let layout = temp_dir.path().join("layout.mag");
    let netlist_cell = "my_netlist_cell".to_string();
    let layout_cell = "my_layout_cell".to_string();
    let ext_path = temp_dir.path().join("netlist_ext.spice");
    let opts = NetgenLvsOpts {
        tech: "sky130A".into(),
    };
    let run_file_path = create_run_file(
        &LvsInput {
            netlist,
            layout,
            netlist_cell,
            layout_cell,
            work_dir: temp_dir.path().to_owned(),
            opts,
        },
        &ext_path,
    )?;

    let output = read_to_string(&run_file_path)?;
    println!("{}", output);

    // Test that the file contains things we expect
    assert!(output.contains("netlist_ext.spice"));
    assert!(output.contains("netlist.spice"));
    assert!(output.contains("my_netlist_cell"));
    assert!(output.contains("my_layout_cell"));
    assert!(output.contains("netgen"));
    assert!(output.contains("lvs"));
    assert!(output.contains("noconsole"));
    assert!(output.contains("full"));
    assert!(output.contains("json"));
    assert!(output.contains("quit"));

    // For netgen, cannot operate directly on the layout
    // Check that the run file does not contain a reference to the layout
    assert!(!output.contains("layout.mag"));
    assert!(!output.contains("layout.gds"));

    Ok(())
}
