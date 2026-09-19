//! The PyO3 arm of the binding-mechanism microbenchmark.
//!
//! It measures exactly what `native/_akmech.c` measures, against the same callee
//! in the same `libakmech_cabi.so`, so the C-extension row and this row differ
//! in the binding mechanism and in nothing else (README R7).
//!
//! README 9.1 calls PyO3 "the same C-API calls with a Rust spelling". That is
//! the claim this arm exists to check rather than to repeat.

use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyString};
use std::os::raw::c_char;

// The forward target, in the plain C library that every mechanism arm reaches.
#[link(name = "akmech_cabi")]
extern "C" {
    fn ak_noop(x: u64) -> u64;
    fn ak_reverse_n(cb: extern "C" fn(u64) -> u64, x: u64, n: usize) -> u64;
    fn ak_reverse_n_floor(x: u64, n: usize) -> u64;
}

/// One forward crossing: Python -> PyO3 -> the C library.
#[pyfunction]
fn fwd_noop(x: u64) -> u64 {
    unsafe { ak_noop(x) }
}

/// The same entry point with the crossing into the C library removed, so the
/// row above can be split into "what PyO3's call machinery costs" and "what the
/// boundary costs".
#[pyfunction]
fn fwd_selfcontained(x: u64) -> u64 {
    x ^ 2
}

// The Python callable a C trampoline re-enters the interpreter to call. A
// `static mut` is the same shape the C arm uses and is safe here because the
// GIL is held for the whole of `rev_python`.
static mut CB: Option<Py<PyAny>> = None;

extern "C" fn trampoline_into_python(x: u64) -> u64 {
    Python::attach(|py| {
        let cb = unsafe {
            let p = &raw const CB;
            match (*p).as_ref() {
                Some(c) => c.bind(py).clone(),
                None => return x,
            }
        };
        match cb.call1((x,)) {
            Ok(r) => r.extract::<u64>().unwrap_or(x),
            Err(_) => x,
        }
    })
}

/// `n` reverse calls into the interpreter, driven from inside the C library.
#[pyfunction]
fn rev_python(cb: Py<PyAny>, n: usize) -> u64 {
    unsafe {
        let p = &raw mut CB;
        (*p) = Some(cb);
    }
    let r = unsafe { ak_reverse_n(trampoline_into_python, 1, n) };
    unsafe {
        let p = &raw mut CB;
        (*p) = None;
    }
    r
}

extern "C" fn plain_cb(x: u64) -> u64 {
    x ^ 2
}

/// `n` C-to-C indirect calls, no interpreter: the floor `rev_python` is quoted
/// above.
#[pyfunction]
fn rev_cfloor(n: usize) -> u64 {
    unsafe { ak_reverse_n(plain_cb, 1, n) }
}

/// The same loop with no call at all.
#[pyfunction]
fn rev_floor(n: usize) -> u64 {
    unsafe { ak_reverse_n_floor(1, n) }
}

/// Diagnostics for the forward row.  PyO3 measured far above the C extension on
/// the same callee, and README R2 says a surprising ratio gets a floor arm
/// before it is reported.  These three split the wrapper's prologue from its
/// argument extraction.
#[pyfunction]
fn fwd_noargs() -> u64 {
    7
}

#[pyfunction]
fn fwd_i64(x: i64) -> i64 {
    x ^ 2
}

#[pyfunction]
fn fwd_any(x: Bound<'_, PyAny>) -> PyResult<u64> {
    let _ = &x;
    Ok(7)
}

const GUID36: &str = "8c8deaf0-3e8d-bdcc-20d5-19afe07cc6c9";
const BLOB16: &[u8] = b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10";

/// `n` repetitions of one primitive, in PyO3's idiom rather than in raw C-API
/// calls. The names match `_akmech.capi` where the operation is the same one,
/// so the two tables can be read side by side.
#[pyfunction]
#[pyo3(signature = (op, n, obj=None, key=None))]
fn capi(
    py: Python<'_>,
    op: &str,
    n: usize,
    obj: Option<Bound<'_, PyAny>>,
    key: Option<Bound<'_, PyAny>>,
) -> PyResult<u64> {
    let mut sink: u64 = 0;
    match op {
        "floor" => {
            for i in 0..n {
                sink = sink.wrapping_add(i as u64);
            }
        }
        "PyLong_FromLongLong" => {
            for i in 0..n {
                let o = (i as i64 + (1 << 20)).into_pyobject(py)?;
                sink = sink.wrapping_add(o.as_ptr() as u64);
            }
        }
        "PyLong_AsLongLong" => {
            let o = obj.as_ref().expect("obj");
            for _ in 0..n {
                sink = sink.wrapping_add(o.extract::<i64>()? as u64);
            }
        }
        "PyUnicode_FromStringAndSize(36)" => {
            for _ in 0..n {
                let o = PyString::new(py, GUID36);
                sink = sink.wrapping_add(o.as_ptr() as u64);
            }
        }
        "PyUnicode_AsUTF8AndSize" => {
            let o = obj.as_ref().expect("obj");
            let s = o.cast::<PyString>()?;
            for _ in 0..n {
                let v = s.to_str()?;
                sink = sink.wrapping_add(v.len() as u64);
            }
        }
        "PyBytes_FromStringAndSize(16)" => {
            for _ in 0..n {
                let o = PyBytes::new(py, BLOB16);
                sink = sink.wrapping_add(o.as_ptr() as u64);
            }
        }
        "PyObject_GetAttr" => {
            let o = obj.as_ref().expect("obj");
            let k = key.as_ref().expect("key");
            let k = k.cast::<PyString>()?;
            for _ in 0..n {
                let v = o.getattr(k)?;
                sink = sink.wrapping_add(v.as_ptr() as u64);
            }
        }
        "PyObject_SetAttr" => {
            let o = obj.as_ref().expect("obj");
            let k = key.as_ref().expect("key");
            let k = k.cast::<PyString>()?;
            for _ in 0..n {
                o.setattr(k, py.None())?;
                sink = sink.wrapping_add(1);
            }
        }
        "PyList_New(4)" => {
            for _ in 0..n {
                let o = PyList::new(py, [py.None(), py.None(), py.None(), py.None()])?;
                sink = sink.wrapping_add(o.as_ptr() as u64);
            }
        }
        "PyList_Append" => {
            let o = obj.as_ref().expect("obj");
            let l = o.cast::<PyList>()?;
            for _ in 0..n {
                l.append(py.None())?;
                sink = sink.wrapping_add(1);
            }
            unsafe {
                ffi::PyList_SetSlice(l.as_ptr(), 0, ffi::PyList_Size(l.as_ptr()), std::ptr::null_mut());
            }
        }
        "PyObject_CallNoArgs(type)" => {
            let o = obj.as_ref().expect("obj");
            for _ in 0..n {
                let v = o.call0()?;
                sink = sink.wrapping_add(v.as_ptr() as u64);
            }
        }
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "unknown op {other}"
            )))
        }
    }
    Ok(sink)
}

#[pyfunction]
fn available(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let d = pyo3::types::PyDict::new(py);
    d.set_item("abi3", cfg!(feature = "abi3"))?;
    d.set_item(
        "ops",
        vec![
            "floor",
            "PyLong_FromLongLong",
            "PyLong_AsLongLong",
            "PyUnicode_FromStringAndSize(36)",
            "PyUnicode_AsUTF8AndSize",
            "PyBytes_FromStringAndSize(16)",
            "PyObject_GetAttr",
            "PyObject_SetAttr",
            "PyList_New(4)",
            "PyList_Append",
            "PyObject_CallNoArgs(type)",
        ],
    )?;
    // Proof the crossing is a real import rather than something the linker
    // folded in: the address of the imported symbol, printed from the artifact
    // side (README R5 is checked from `nm -D` in build.sh as well).
    d.set_item("ak_noop_addr", ak_noop as *const c_char as usize)?;
    Ok(d.into_any().unbind())
}

#[pymodule]
fn _akmech_pyo3(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fwd_noop, m)?)?;
    m.add_function(wrap_pyfunction!(fwd_selfcontained, m)?)?;
    m.add_function(wrap_pyfunction!(fwd_noargs, m)?)?;
    m.add_function(wrap_pyfunction!(fwd_i64, m)?)?;
    m.add_function(wrap_pyfunction!(fwd_any, m)?)?;
    m.add_function(wrap_pyfunction!(rev_python, m)?)?;
    m.add_function(wrap_pyfunction!(rev_cfloor, m)?)?;
    m.add_function(wrap_pyfunction!(rev_floor, m)?)?;
    m.add_function(wrap_pyfunction!(capi, m)?)?;
    m.add_function(wrap_pyfunction!(available, m)?)?;
    Ok(())
}
