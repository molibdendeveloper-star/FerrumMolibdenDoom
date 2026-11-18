use crate::d_main::D_DoomMain;
use crate::m_argv::set_args;

pub fn run_doom(args: Vec<String>) {
    set_args(args);
    D_DoomMain();
}
