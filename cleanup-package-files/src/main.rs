#[macro_use] extern crate failure;
extern crate nix;
extern crate expanduser;
extern crate structopt;
extern crate pwd;

use std::process::Command;
use std::path::Path;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::fs::File;
use std::os::unix::io::IntoRawFd;

use expanduser::expanduser;
use failure::Error;
use structopt::StructOpt;

const LILAC_LOCK: &str = "~lilydjwg/.lilac/.lock";
const LILAC_REPO: &str = "~lilydjwg/archgitrepo";
const USER: &str = "lilydjwg";

fn flock<P: AsRef<Path>>(lockfile: P) -> Result<(), Error> {
  let f = File::open(lockfile)?;
  nix::fcntl::flock(f.into_raw_fd(), nix::fcntl::FlockArg::LockExclusive)?;
  Ok(())
}

fn git_ls_files() -> Result<Vec<OsString>, Error> {
  let output = Command::new("git").arg("ls-files").output()?;
  ensure!(output.status.success(),
    "{:?}: {}", output.status, String::from_utf8_lossy(&output.stderr));
  Ok(
    output.stdout.split(|c| *c == b'\n')
      .map(|line| OsStr::from_bytes(line).to_owned())
      .collect()
  )
}

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
  /// remove files for real; or only print what will be removed
  #[structopt(long="real")]
  real: bool,
  pkgname: String,
}

fn main() -> Result<(),Error> {
  let opt = Opt::from_args();

  let pwd = pwd::Passwd::from_name(USER)?.unwrap();
  nix::unistd::setuid(nix::unistd::Uid::from_raw(pwd.uid))?;

  flock(&expanduser(LILAC_LOCK)?)?;
  let mut path = expanduser(LILAC_REPO)?;
  path.push(&opt.pkgname);

  std::env::set_current_dir(&path)?;
  let tracked_files = git_ls_files()?;

  for entry in path.read_dir()? {
    let entry = entry?;
    let file_name = entry.file_name();
    if tracked_files.contains(&file_name) {
      continue;
    }
    if opt.real {
      println!("rm -rf {}", entry.path().display());
      Command::new("rm").arg("-rf").arg(&file_name).spawn()?;
    } else {
      println!("Would rm -rf {}", entry.path().display());
    }
  }

  Ok(())
}
