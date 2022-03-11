use magic_vlsi::MagicInstance;

use crate::verification::lvs::{Lvs, LvsOpts, LvsError, LvsOutput, LvsInput};

#[derive(Debug)]
pub struct MagicLvs {
}

impl MagicLvs {
    pub fn new() -> Self {
        Self {}
    }
}

impl Lvs<LvsOpts, LvsError> for MagicLvs {
    fn lvs(input: LvsInput<LvsOpts>) -> LvsOutput<LvsError> {
        let mut m = MagicInstance::builder()
            .cwd(input.work_dir)
            .tech("sky130A")
            .build()
            .unwrap();
        LvsOutput {
            ok: false,
            errors: vec![],
        }
    }
}
