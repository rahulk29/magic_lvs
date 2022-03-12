use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::prelude::OpenOptionsExt,
    path::{Path, PathBuf},
};

use magic_vlsi::MagicInstance;
use serde::Serialize;

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
        let ext_path = extract(&input)?;
        // Run netgen on the resulting netlist
        run_netgen(&input, &ext_path)?;

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
    m.load(&cell_path)?;

    m.exec_one("ext2spice lvs")?;
    m.exec_one("ext2spice format ngspice")?;
    m.exec_one("ext")?;
    m.exec_one("ext2spice")?;

    Ok(input.work_dir.join(format!("{}.spice", &input.layout_cell)))
}

fn run_netgen(input: &LvsInput<NetgenLvsOpts>, ext_path: impl AsRef<Path>) -> Result<()> {
    let ext_path = ext_path.as_ref();
    create_run_file(input, ext_path)?;
    Ok(())
}

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

fn create_run_file(input: &LvsInput<NetgenLvsOpts>, ext_path: &Path) -> Result<PathBuf> {
    let mut hbs = handlebars::Handlebars::new();
    hbs.register_template_string(RUN_LVS_FILENAME, RUN_LVS_TEMPLATE)?;

    let run_file_opts = RunFileOpts {
        netlist: input.netlist.clone(),
        netlist_cell: input.netlist_cell.clone(),
        layout: ext_path.to_owned(),
        layout_cell: input.layout_cell.clone(),
        setup_file: PathBuf::from(SETUP_FILE_NAME),
        output_file: PathBuf::from(OUT_FILE_NAME),
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

    Ok(run_file_path)
}
