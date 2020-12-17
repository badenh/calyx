//! Centralized error handling for FuTIL. Each variant of the
//! `Error` enum represents a different type of error. For some types of errors, you
//! might want to add a `From` impl so that the `?` syntax is more convienent.

use crate::frontend::{ast, library, parser};
use crate::ir;
use petgraph::stable_graph::NodeIndex;
use std::iter::repeat;
use std::rc::Rc;

/// Standard error type for FuTIL errors.
#[allow(clippy::large_enum_variant)]
pub enum Error {
    /// Error while parsing a FuTIL program.
    ParseError(pest_consume::Error<parser::Rule>),
    /// Error while parsing a FuTIL library.
    LibraryParseError(pest_consume::Error<library::parser::Rule>),
    /// Using a reserved keyword as a program identifier.
    ReservedName(ir::Id),

    /// The given string does not correspond to any known pass.
    UnknownPass(String, String),
    /// The input file is invalid (does not exist).
    InvalidFile(String),
    /// Failed to write the output
    WriteError,

    /// The control program is malformed.
    MalformedControl(String),

    /// The connections are malformed.
    MalformedStructure(String),
    /// The port widths don't match up on an edge.
    MismatchedPortWidths(ast::Port, u64, ast::Port, u64),

    /// The name has not been bound
    Undefined(ir::Id, String),
    /// The name has already been bound.
    AlreadyBound(ir::Id, String),

    /// The group was not used in the program.
    UnusedGroup(ir::Id),

    /// No value provided for a primitive parameter.
    SignatureResolutionFailed(ir::Id, ir::Id),

    /// An implementation is missing.
    MissingImplementation(&'static str, ir::Id),

    /// Papercut error: signals a commonly made mistake in FuTIL program.
    Papercut(String, ir::Id),

    /// Group "static" latency annotation differed from inferred latency.
    ImpossibleLatencyAnnotation(String, u64, u64),

    /// Internal compiler error that should never occur.
    Impossible(String), // Signal compiler errors that should never occur.
    NotSubcomponent,

    /// A miscellaneous error. Should be replaced with a more precise error.
    #[allow(unused)]
    Misc(String),
}

/// Convience wrapper to represent success or meaningul compiler error.
pub type FutilResult<T> = std::result::Result<T, Error>;

/// A span of the input program.
/// Used for reporting location-based errors.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    /// Reference to input program source.
    input: Rc<str>,
    /// The start of the span.
    start: usize,
    /// The end of the span.
    end: usize,
}

impl Span {
    /// Create a new `Error::Span` from a `pest::Span` and
    /// the input string.
    pub fn new(span: pest::Span, input: Rc<str>) -> Span {
        Span {
            input,
            start: span.start(),
            end: span.end(),
        }
    }

    /// Format this Span with a the error message `err_msg`
    pub fn format(&self, err_msg: &str) -> String {
        let lines = self.input.split('\n');
        let mut buf: String = String::new();
        let mut pos: usize = 0;
        let mut linum: usize = 1;
        for l in lines {
            let new_pos = pos + l.len() + 1;
            if self.start > pos && self.end < pos + (l.len()) {
                let linum_text = format!("{} ", linum);
                let linum_space: String =
                    repeat(" ").take(linum_text.len()).collect();
                let mark: String =
                    repeat("^").take(self.end - self.start).collect();
                let space: String =
                    repeat(" ").take(self.start - pos).collect();
                buf += "\n";
                buf += &format!("{}|{}\n", linum_text, l);
                buf +=
                    &format!("{}|{}{} {}", linum_space, space, mark, err_msg);
                break;
            }
            pos = new_pos;
            linum += 1;
        }
        buf
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use Error::*;
        match self {
            Papercut(msg, id) => {
                write!(f, "{}", id.fmt_err(&("[Papercut] ".to_string() + msg)))
            }
            ImpossibleLatencyAnnotation(grp_name, ann_val, inferred_val) => {
                let msg1 = format!("Annotated latency: {}", ann_val);
                let msg2 = format!("Inferred latency: {}", inferred_val);
                write!(
                    f,
                    "Impossible \"static\" latency annotation for group {}.\n{}\n{}",
                    grp_name,
                    msg1,
                    msg2
                )
            }
            UnusedGroup(name) => {
                write!(
                    f,
                    "{}",
                    name.fmt_err("Group not used in control")
                )
            }
            AlreadyBound(name, bound_by) => {
                let msg = format!("Name already bound by {}", bound_by.to_string());
                write!(f, "{}", name.fmt_err(&msg))
            }
            ReservedName(name) => {
                let msg = format!("Use of reserved keyword: {}", name.to_string());
                write!(f, "{}", name.fmt_err(&msg))
            }
            Undefined(name, typ) => {
                let msg = format!("Undefined {} name: {}", typ, name.to_string());
                write!(
                    f,
                    "{}",
                    name.fmt_err(&msg)
                )
            }
            UnknownPass(pass, known_passes) => {
                write!(
                    f,
                    "Unknown pass: {}. Known passes: {}.",
                    pass,
                    known_passes
                )
            },
            InvalidFile(err) => write!(f, "InvalidFile: {}", err),
            ParseError(err) => write!(f, "FuTIL Parser: {}", err),
            LibraryParseError(err) => write!(f, "FuTIL Library Parser: {}", err),
            WriteError => write!(f, "WriteError"),
            MismatchedPortWidths(port1, w1, port2, w2) => {
                let msg1 = format!("This port has width: {}", w1);
                let msg2 = format!("This port has width: {}", w2);
                write!(f, "{}\nwhich doesn't match the width of '{}':{}",
                       port1.port_name().fmt_err(&msg1),
                       port2.port_name().to_string(),
                       port2.port_name().fmt_err(&msg2))
            }
            SignatureResolutionFailed(id, param_name) => {
                let msg = format!("No value passed in for parameter: {}", param_name.to_string());
                write!(f, "{}\nwhich is used here:{}", id.fmt_err(&msg), param_name.fmt_err(""))
            }
            MalformedControl(msg) => write!(f, "Malformed Control: {}", msg),
            MalformedStructure(msg) => write!(f, "Malformed Structure: {}", msg),
            NotSubcomponent => write!(f, "Not a subcomponent"),
            Misc(msg) => write!(f, "{}", msg),
            Impossible(msg) => write!(f, "Impossible: {}\nThis error should never occur. Report report this as a bug.", msg),
            MissingImplementation(name, id) => write!(f, "Mising {} implementation for `{}`", name, id.to_string())
        }
    }
}

// Conversions from other error types to our error type so that
// we can use `?` in all the places.

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::InvalidFile(err.to_string())
    }
}

impl From<std::fmt::Error> for Error {
    fn from(_err: std::fmt::Error) -> Self {
        Error::WriteError
    }
}

impl From<pest_consume::Error<parser::Rule>> for Error {
    fn from(e: pest_consume::Error<parser::Rule>) -> Self {
        Error::ParseError(e)
    }
}

impl From<pest_consume::Error<library::parser::Rule>> for Error {
    fn from(e: pest_consume::Error<library::parser::Rule>) -> Self {
        Error::LibraryParseError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(_e: std::io::Error) -> Self {
        Error::WriteError
    }
}

// Utility traits

/// A generalized 'unwrapping' trait that extracts data from
/// a container that can possible be an error and automatically
/// generates the correct `Error` variant with the `ir::Id`.
/// For example, `Extract<NodeIndex, NodeIndex>` can be implemented for
/// `Option<NodeIndex>` to provide convienent error reporting for
/// undefined components / groups.
pub trait Extract<T, R> {
    /// Unpacks `T` into `FutilResult<R>` using `id: ir::Id`
    /// for error reporting with locations.
    fn extract(&self, id: &ir::Id) -> FutilResult<R>;
}

impl Extract<NodeIndex, NodeIndex> for Option<NodeIndex> {
    fn extract(&self, id: &ir::Id) -> FutilResult<NodeIndex> {
        match self {
            Some(t) => Ok(*t),
            None => Err(Error::Undefined(id.clone(), "component".to_string())),
        }
    }
}
