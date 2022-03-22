use std::collections::HashMap;
use std::{fs::read_to_string, path::PathBuf};

use crate::protos::lvs::LvsInput;
use crate::{lvs::Lvs, protos::lvs::LvsTool};

use super::{create_run_file, NetgenLvs};

#[test]
fn test_create_run_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let netlist_path = temp_dir
        .path()
        .join("netlist.spice")
        .into_os_string()
        .into_string()
        .unwrap();
    let layout_path = temp_dir
        .path()
        .join("layout.mag")
        .into_os_string()
        .into_string()
        .unwrap();
    let netlist_cell = "my_netlist_cell".to_string();
    let layout_cell = "my_layout_cell".to_string();
    let ext_path = temp_dir.path().join("netlist_ext.spice");
    let (run_file_path, _) = create_run_file(
        &LvsInput {
            netlist_path,
            layout_path,
            netlist_cell,
            layout_cell,
            tool: LvsTool::MagicNetgen as i32,
            tech: "sky130".to_string(),
            options: HashMap::default(),
        },
        temp_dir.path(),
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

#[test]
fn test_lvs_sky130_clean() -> Result<(), Box<dyn std::error::Error>> {
    println!("lvs_sky130_clean beginning");
    let work_dir: PathBuf = "/tmp/sram22/tests/lvs/clean".into();
    std::fs::create_dir_all(&work_dir)?;
    println!("done creating dirs");
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let netlist_path = base
        .join("src/plugins/netgen_lvs/tests/data/clean/nand2.spice")
        .into_os_string()
        .into_string()
        .unwrap();
    let layout_path = base
        .join("src/plugins/netgen_lvs/tests/data/clean/nand2_dec_auto.mag")
        .into_os_string()
        .into_string()
        .unwrap();
    NetgenLvs::new().lvs(
        LvsInput {
            netlist_path,
            layout_path,
            tool: LvsTool::MagicNetgen as i32,
            tech: "sky130".to_string(),
            options: HashMap::default(),
            netlist_cell: "nand2_n420x150_p420x150".to_string(),
            layout_cell: "nand2_dec_auto".to_string(),
        },
        work_dir,
    )?;
    Ok(())
}
