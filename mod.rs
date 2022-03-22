use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Write},
    os::unix::prelude::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use magic_vlsi::MagicInstance;
use serde::Serialize;

use crate::{
    error::Result,
    lvs::Lvs,
    protos::lvs::{LvsInput, LvsOutput},
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct NetgenLvs {}

impl NetgenLvs {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for NetgenLvs {
    fn default() -> Self {
        Self::new()
    }
}

impl Lvs for NetgenLvs {
    fn lvs(&self, input: LvsInput, work_dir: PathBuf) -> Result<LvsOutput> {
        std::fs::create_dir_all(&work_dir)?;
        // Extract the layout into a netlist using MAGIC
        let ext_path = extract(&input, &work_dir)?;
        // Run netgen on the resulting netlist
        run_netgen(&input, &work_dir, &ext_path)?;

        Ok(LvsOutput {
            matches: false,
            errors: vec![],
            warnings: vec![],
        })
    }
}

fn extract(input: &LvsInput, work_dir: impl AsRef<Path>) -> Result<PathBuf> {
    let tech = match input.tech.as_str() {
        "sky130" => "sky130A",
        x => x,
    };
    let mut m = MagicInstance::builder()
        .cwd(&work_dir)
        .tech(tech)
        .port(portpicker::pick_unused_port().expect("no free ports"))
        .build()
        .unwrap();

    m.drc_off()?;
    m.set_snap(magic_vlsi::SnapMode::Internal)?;

    println!("loading cell {}", &input.layout_path);
    m.load(&input.layout_path)?;

    m.exec_one("ext2spice lvs")?;
    m.exec_one("ext2spice format ngspice")?;
    m.exec_one("ext")?;
    m.exec_one("ext2spice")?;

    Ok(work_dir
        .as_ref()
        .join(format!("{}.spice", &input.layout_cell)))
}

fn run_netgen(
    input: &LvsInput,
    work_dir: impl AsRef<Path>,
    ext_path: impl AsRef<Path>,
) -> Result<()> {
    let ext_path = ext_path.as_ref();
    create_setup_file(input, &work_dir)?;
    let (run_file, out_file) = create_run_file(input, &work_dir, ext_path)?;
    execute_run_file(&work_dir, &run_file)?;
    let _ = parse_lvs_results(&out_file)?;
    Ok(())
}

const SKY130_SETUP_FILE: &str = include_str!("tech/sky130/setup.tcl");
const RUN_LVS_TEMPLATE: &str = include_str!("run_lvs.sh.hbs");
const RUN_LVS_FILENAME: &str = "run_lvs.sh";
const SETUP_FILE_NAME: &str = "edatool_netgen_lvs_setup.tcl";
const OUT_FILE_NAME: &str = "edatool_netgen_lvs_comp.json";

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
struct RunFileOpts {
    netlist: PathBuf,
    netlist_cell: String,
    layout: PathBuf,
    layout_cell: String,
    setup_file: PathBuf,
    output_file: PathBuf,
}

fn create_run_file(
    input: &LvsInput,
    work_dir: impl AsRef<Path>,
    ext_path: &Path,
) -> Result<(PathBuf, PathBuf)> {
    let mut hbs = handlebars::Handlebars::new();
    hbs.register_template_string(RUN_LVS_FILENAME, RUN_LVS_TEMPLATE)?;

    let output_file = work_dir.as_ref().join(OUT_FILE_NAME);

    let run_file_opts = RunFileOpts {
        netlist: PathBuf::from(&input.netlist_path),
        netlist_cell: input.netlist_cell.clone(),
        layout: ext_path.to_owned(),
        layout_cell: input.layout_cell.clone(),
        setup_file: PathBuf::from(SETUP_FILE_NAME),
        output_file: output_file.clone(),
    };

    let run_file_path = work_dir.as_ref().join(RUN_LVS_FILENAME);

    // Render the template to the file
    {
        let mut options = OpenOptions::new();
        let mut run_file = options
            .mode(0o744)
            .write(true)
            .truncate(true)
            .create(true)
            .read(false)
            .open(&run_file_path)?;
        hbs.render_to_write(RUN_LVS_FILENAME, &run_file_opts, &mut run_file)?;
        run_file.flush()?;
    }

    Ok((run_file_path, output_file))
}

/// Creates a TCL file containing netgen setup commands
fn create_setup_file(input: &LvsInput, work_dir: impl AsRef<Path>) -> Result<PathBuf> {
    let path = work_dir.as_ref().join(SETUP_FILE_NAME);
    {
        let mut file = File::create(&path)?;
        match input.tech.as_str() {
            "sky130" => {
                write!(&mut file, "{}", SKY130_SETUP_FILE)?;
            }
            _ => unimplemented!("netgen lvs does not support tech {}", input.tech),
        };
        file.flush()?;
    }
    Ok(path)
}

/// Executes the specified run file
fn execute_run_file(cwd: impl AsRef<Path>, file: impl AsRef<Path>) -> Result<()> {
    Command::new(file.as_ref())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(cwd)
        .status()?;
    Ok(())
}

/// Parses the JSON output file
fn parse_lvs_results(path: impl AsRef<Path>) -> Result<()> {
    println!("opening lvs results at {:?}", path.as_ref());
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let _: serde_json::Value = serde_json::from_reader(reader)?;
    Ok(())
}
