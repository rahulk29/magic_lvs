use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Write},
    os::unix::prelude::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use magic_vlsi::MagicInstance;
use serde::{Deserialize, Serialize};

use crate::{
    error::Result,
    verification::lvs::{Lvs, LvsError, LvsInput, LvsOutput},
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct NetgenLvs {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NetgenLvsOpts {
    /// The name of the technology to use when running magic
    pub tech: String,
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq)]
struct NetgenOutput {
    runs: Vec<NetgenComparison>,
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq)]
struct NetgenComparison {}

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

impl Lvs<NetgenLvsOpts, LvsError> for NetgenLvs {
    fn lvs(&self, input: LvsInput<NetgenLvsOpts>) -> Result<LvsOutput<LvsError>> {
        // Extract the layout into a netlist using MAGIC
        println!("ext");
        let ext_path = extract(&input)?;
        println!("ext_done");
        // Run netgen on the resulting netlist
        run_netgen(&input, &ext_path)?;
        println!("netgen_done");

        Ok(LvsOutput {
            ok: false,
            errors: vec![],
        })
    }
}

fn extract(input: &LvsInput<NetgenLvsOpts>) -> Result<PathBuf> {
    let mut m = MagicInstance::builder()
        .cwd(&input.work_dir)
        .tech("sky130A")
        .port(portpicker::pick_unused_port().expect("no free ports"))
        .build()
        .unwrap();

    m.drc_off()?;
    m.set_snap(magic_vlsi::SnapMode::Internal)?;

    let cell_path = input
        .layout
        .to_owned()
        .into_os_string()
        .into_string()
        .unwrap();
    println!("loading cell {}", &cell_path);
    m.load(&cell_path)?;

    m.exec_one("ext2spice lvs")?;
    m.exec_one("ext2spice format ngspice")?;
    m.exec_one("ext")?;
    m.exec_one("ext2spice")?;

    Ok(input.work_dir.join(format!("{}.spice", &input.layout_cell)))
}

fn run_netgen(input: &LvsInput<NetgenLvsOpts>, ext_path: impl AsRef<Path>) -> Result<()> {
    let ext_path = ext_path.as_ref();
    create_setup_file(input)?;
    println!("here 1");
    let (run_file, out_file) = create_run_file(input, ext_path)?;
    println!("here 2");
    execute_run_file(&input.work_dir, &run_file)?;
    println!("here 3");
    let _ = parse_lvs_results(&out_file)?;
    println!("here 4");
    Ok(())
}

const SKY130_SETUP_FILE: &str = include_str!("tech/sky130/setup.tcl");
const RUN_LVS_TEMPLATE: &str = include_str!("run_lvs.sh.hbs");
const RUN_LVS_FILENAME: &str = "run_lvs.sh";
const SETUP_FILE_NAME: &str = "sram22_netgen_lvs_setup.tcl";
const OUT_FILE_NAME: &str = "sram22_netgen_lvs_comp.json";

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
struct RunFileOpts {
    netlist: PathBuf,
    netlist_cell: String,
    layout: PathBuf,
    layout_cell: String,
    setup_file: PathBuf,
    output_file: PathBuf,
}

fn create_run_file(input: &LvsInput<NetgenLvsOpts>, ext_path: &Path) -> Result<(PathBuf, PathBuf)> {
    let mut hbs = handlebars::Handlebars::new();
    hbs.register_template_string(RUN_LVS_FILENAME, RUN_LVS_TEMPLATE)?;

    let output_file = input.work_dir.join(OUT_FILE_NAME);

    let run_file_opts = RunFileOpts {
        netlist: input.netlist.clone(),
        netlist_cell: input.netlist_cell.clone(),
        layout: ext_path.to_owned(),
        layout_cell: input.layout_cell.clone(),
        setup_file: PathBuf::from(SETUP_FILE_NAME),
        output_file: output_file.clone(),
    };

    let run_file_path = input.work_dir.join(RUN_LVS_FILENAME);

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
fn create_setup_file(input: &LvsInput<NetgenLvsOpts>) -> Result<PathBuf> {
    let path = input.work_dir.join(SETUP_FILE_NAME);
    {
        let mut file = File::create(&path)?;
        match input.opts.tech.as_str() {
            "sky130" => {
                write!(&mut file, "{}", SKY130_SETUP_FILE)?;
            }
            _ => unimplemented!("netgen lvs does not support tech {}", input.opts.tech),
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
