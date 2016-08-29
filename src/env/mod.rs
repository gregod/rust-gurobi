// Copyright (c) 2016 Yusuke Sasaki
//
// This software is released under the MIT License.
// See http://opensource.org/licenses/mit-license.php or <LICENSE>.

pub mod param;

use ffi;

use std::ffi::CString;
use std::ptr::{null, null_mut};
use std::rc::Rc;

use error::{Error, Result};
use model::Model;
use util;

/// internal data for Env.
struct EnvRep {
  ptr: *mut ffi::GRBenv
}

impl Drop for EnvRep {
  fn drop(&mut self) {
    unsafe { ffi::GRBfreeenv(self.ptr) };
    self.ptr = null_mut();
  }
}


/// Gurobi environment object
#[derive(Clone)]
pub struct Env(Rc<EnvRep>);

impl Env {
  /// Create an environment with log file
  pub fn new(logfilename: &str) -> Result<Env> {
    let mut env = null_mut();
    let logfilename = try!(CString::new(logfilename));
    let error = unsafe { ffi::GRBloadenv(&mut env, logfilename.as_ptr()) };
    if error != 0 {
      return Err(Error::FromAPI(get_error_msg(env), error));
    }
    Ok(Env::from_raw(env))
  }

  /// Create a client environment on a computer server with log file
  pub fn new_client(logfilename: &str, computeserver: &str, port: i32, password: &str, priority: i32, timeout: f64)
                -> Result<Env> {
    let mut env = null_mut();
    let logfilename = try!(CString::new(logfilename));
    let computeserver = try!(CString::new(computeserver));
    let password = try!(CString::new(password));
    let error = unsafe {
      ffi::GRBloadclientenv(&mut env,
                            logfilename.as_ptr(),
                            computeserver.as_ptr(),
                            port,
                            password.as_ptr(),
                            priority,
                            timeout)
    };
    if error != 0 {
      return Err(Error::FromAPI(get_error_msg(env), error));
    }
    Ok(Env::from_raw(env))
  }

  /// Create an empty model object associted with the environment
  pub fn new_model(&self, modelname: &str) -> Result<Model> {
    let modelname = try!(CString::new(modelname));
    let mut model = null_mut();
    try!(self.check_apicall(unsafe {
      ffi::GRBnewmodel(self.0.ptr,
                       &mut model,
                       modelname.as_ptr(),
                       0,
                       null(),
                       null(),
                       null(),
                       null(),
                       null())
    }));
    Model::new(self.clone(), model)
  }

  /// Read a model from a file
  pub fn read_model(&self, filename: &str) -> Result<Model> {
    let filename = try!(CString::new(filename));
    let mut model = null_mut();
    try!(self.check_apicall(unsafe { ffi::GRBreadmodel(self.0.ptr, filename.as_ptr(), &mut model) }));
    Model::new(self.clone(), model)
  }

  /// Query the value of a parameter
  pub fn get<P: param::ParamBase>(&self, param: P) -> Result<P::Out> {
    use util::AsRawPtr;
    let mut value: P::Buf = util::Init::init();
    try!(self.check_apicall(unsafe { P::get_param(self.0.ptr, param.into().as_ptr(), value.as_rawptr()) }));

    Ok(util::Into::into(value))
  }

  /// Set the value of a parameter
  pub fn set<P: param::ParamBase>(&mut self, param: P, value: P::Out) -> Result<()> {
    self.check_apicall(unsafe {
      P::set_param(self.0.ptr,
                   param.into().as_ptr(),
                   util::FromRaw::from(value))
    })
  }

  /// Import a set of parameter values from a file
  pub fn read_params(&mut self, filename: &str) -> Result<()> {
    let filename = try!(CString::new(filename));
    self.check_apicall(unsafe { ffi::GRBreadparams(self.0.ptr, filename.as_ptr()) })
  }

  /// Write the set of parameter values to a file
  pub fn write_params(&self, filename: &str) -> Result<()> {
    let filename = try!(CString::new(filename));
    self.check_apicall(unsafe { ffi::GRBwriteparams(self.0.ptr, filename.as_ptr()) })
  }

  /// Insert a message into log file.
  ///
  /// When **message** cannot convert to raw C string, a panic is occurred.
  pub fn message(&self, message: &str) { unsafe { ffi::GRBmsg(self.0.ptr, CString::new(message).unwrap().as_ptr()) }; }

  fn check_apicall(&self, error: ffi::c_int) -> Result<()> {
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }
}

pub trait FromRaw {
  fn from_raw(env: *mut ffi::GRBenv) -> Self;
}

impl FromRaw for Env {
  fn from_raw(ptr: *mut ffi::GRBenv) -> Env { Env(Rc::new(EnvRep { ptr: ptr })) }
}


pub trait ErrorFromAPI {
  fn error_from_api(&self, error: ffi::c_int) -> Error;
}

impl ErrorFromAPI for Env {
  fn error_from_api(&self, error: ffi::c_int) -> Error { Error::FromAPI(get_error_msg(self.0.ptr), error) }
}


fn get_error_msg(env: *mut ffi::GRBenv) -> String { unsafe { util::from_c_str(ffi::GRBgeterrormsg(env)) } }


// #[test]
// fn env_with_logfile() {
//   use std::path::Path;
//   use std::fs::remove_file;
//
//   let path = Path::new("test_env.log");
//
//   if path.exists() {
//     remove_file(path).unwrap();
//   }
//
//   {
//     let env = Env::new(path.to_str().unwrap()).unwrap();
//   }
//
//   assert!(path.exists());
//   remove_file(path).unwrap();
// }

#[test]
fn param_accesors_should_be_valid() {
  use super::param;
  let mut env = Env::new("").unwrap();
  env.set(param::IISMethod, 1).unwrap();
  let iis_method = env.get(param::IISMethod).unwrap();
  assert_eq!(iis_method, 1);
}