use crate::diagnostic::DiagnosticLocation;
use crate::{fluent_generated as fluent, AddToDiagnostic};
use crate::{
    DiagCtxt, DiagnosticArgValue, DiagnosticBuilder, EmissionGuarantee, IntoDiagnostic,
    IntoDiagnosticArg, Level,
};
use rustc_ast as ast;
use rustc_ast_pretty::pprust;
use rustc_hir as hir;
use rustc_span::edition::Edition;
use rustc_span::symbol::{Ident, MacroRulesNormalizedIdent, Symbol};
use rustc_span::Span;
use rustc_target::abi::TargetDataLayoutErrors;
use rustc_target::spec::{PanicStrategy, SplitDebuginfo, StackProtector, TargetTriple};
use rustc_type_ir as type_ir;
use std::backtrace::Backtrace;
use std::borrow::Cow;
use std::fmt;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

pub struct DiagnosticArgFromDisplay<'a>(pub &'a dyn fmt::Display);

impl IntoDiagnosticArg for DiagnosticArgFromDisplay<'_> {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        self.0.to_string().into_diagnostic_arg()
    }
}

impl<'a> From<&'a dyn fmt::Display> for DiagnosticArgFromDisplay<'a> {
    fn from(t: &'a dyn fmt::Display) -> Self {
        DiagnosticArgFromDisplay(t)
    }
}

impl<'a, T: fmt::Display> From<&'a T> for DiagnosticArgFromDisplay<'a> {
    fn from(t: &'a T) -> Self {
        DiagnosticArgFromDisplay(t)
    }
}

impl<'a, T: Clone + IntoDiagnosticArg> IntoDiagnosticArg for &'a T {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        self.clone().into_diagnostic_arg()
    }
}

macro_rules! into_diagnostic_arg_using_display {
    ($( $ty:ty ),+ $(,)?) => {
        $(
            impl IntoDiagnosticArg for $ty {
                fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
                    self.to_string().into_diagnostic_arg()
                }
            }
        )+
    }
}

into_diagnostic_arg_using_display!(
    ast::ParamKindOrd,
    i8,
    u8,
    i16,
    u16,
    u32,
    i64,
    i128,
    u128,
    std::io::Error,
    Box<dyn std::error::Error>,
    std::num::NonZeroU32,
    hir::Target,
    Edition,
    Ident,
    MacroRulesNormalizedIdent,
    ParseIntError,
    StackProtector,
    &TargetTriple,
    SplitDebuginfo,
    ExitStatus,
);

impl IntoDiagnosticArg for i32 {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Number(self.into())
    }
}

impl IntoDiagnosticArg for u64 {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Number(self.into())
    }
}

impl IntoDiagnosticArg for bool {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        if self {
            DiagnosticArgValue::Str(Cow::Borrowed("true"))
        } else {
            DiagnosticArgValue::Str(Cow::Borrowed("false"))
        }
    }
}

impl IntoDiagnosticArg for char {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Owned(format!("{self:?}")))
    }
}

impl IntoDiagnosticArg for Symbol {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        self.to_ident_string().into_diagnostic_arg()
    }
}

impl<'a> IntoDiagnosticArg for &'a str {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        self.to_string().into_diagnostic_arg()
    }
}

impl IntoDiagnosticArg for String {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Owned(self))
    }
}

impl<'a> IntoDiagnosticArg for Cow<'a, str> {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Owned(self.into_owned()))
    }
}

impl<'a> IntoDiagnosticArg for &'a Path {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Owned(self.display().to_string()))
    }
}

impl IntoDiagnosticArg for PathBuf {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Owned(self.display().to_string()))
    }
}

impl IntoDiagnosticArg for usize {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Number(self as i128)
    }
}

impl IntoDiagnosticArg for PanicStrategy {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Owned(self.desc().to_string()))
    }
}

impl IntoDiagnosticArg for hir::ConstContext {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Borrowed(match self {
            hir::ConstContext::ConstFn => "const_fn",
            hir::ConstContext::Static(_) => "static",
            hir::ConstContext::Const { .. } => "const",
        }))
    }
}

impl IntoDiagnosticArg for ast::Expr {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Owned(pprust::expr_to_string(&self)))
    }
}

impl IntoDiagnosticArg for ast::Path {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Owned(pprust::path_to_string(&self)))
    }
}

impl IntoDiagnosticArg for ast::token::Token {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(pprust::token_to_string(&self))
    }
}

impl IntoDiagnosticArg for ast::token::TokenKind {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(pprust::token_kind_to_string(&self))
    }
}

impl IntoDiagnosticArg for type_ir::FloatTy {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Borrowed(self.name_str()))
    }
}

impl IntoDiagnosticArg for std::ffi::CString {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Owned(self.to_string_lossy().into_owned()))
    }
}

impl IntoDiagnosticArg for rustc_data_structures::small_c_str::SmallCStr {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Owned(self.to_string_lossy().into_owned()))
    }
}

impl IntoDiagnosticArg for ast::Visibility {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        let s = pprust::vis_to_string(&self);
        let s = s.trim_end().to_string();
        DiagnosticArgValue::Str(Cow::Owned(s))
    }
}

impl IntoDiagnosticArg for rustc_lint_defs::Level {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Borrowed(self.to_cmd_flag()))
    }
}

#[derive(Clone)]
pub struct DiagnosticSymbolList(Vec<Symbol>);

impl From<Vec<Symbol>> for DiagnosticSymbolList {
    fn from(v: Vec<Symbol>) -> Self {
        DiagnosticSymbolList(v)
    }
}

impl IntoDiagnosticArg for DiagnosticSymbolList {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::StrListSepByAnd(
            self.0.into_iter().map(|sym| Cow::Owned(format!("`{sym}`"))).collect(),
        )
    }
}

impl<Id> IntoDiagnosticArg for hir::def::Res<Id> {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::Borrowed(self.descr()))
    }
}

impl<G: EmissionGuarantee> IntoDiagnostic<'_, G> for TargetDataLayoutErrors<'_> {
    fn into_diagnostic(self, dcx: &DiagCtxt, level: Level) -> DiagnosticBuilder<'_, G> {
        match self {
            TargetDataLayoutErrors::InvalidAddressSpace { addr_space, err, cause } => {
                DiagnosticBuilder::new(dcx, level, fluent::errors_target_invalid_address_space)
                    .arg_mv("addr_space", addr_space)
                    .arg_mv("cause", cause)
                    .arg_mv("err", err)
            }
            TargetDataLayoutErrors::InvalidBits { kind, bit, cause, err } => {
                DiagnosticBuilder::new(dcx, level, fluent::errors_target_invalid_bits)
                    .arg_mv("kind", kind)
                    .arg_mv("bit", bit)
                    .arg_mv("cause", cause)
                    .arg_mv("err", err)
            }
            TargetDataLayoutErrors::MissingAlignment { cause } => {
                DiagnosticBuilder::new(dcx, level, fluent::errors_target_missing_alignment)
                    .arg_mv("cause", cause)
            }
            TargetDataLayoutErrors::InvalidAlignment { cause, err } => {
                DiagnosticBuilder::new(dcx, level, fluent::errors_target_invalid_alignment)
                    .arg_mv("cause", cause)
                    .arg_mv("err_kind", err.diag_ident())
                    .arg_mv("align", err.align())
            }
            TargetDataLayoutErrors::InconsistentTargetArchitecture { dl, target } => {
                DiagnosticBuilder::new(dcx, level, fluent::errors_target_inconsistent_architecture)
                    .arg_mv("dl", dl)
                    .arg_mv("target", target)
            }
            TargetDataLayoutErrors::InconsistentTargetPointerWidth { pointer_size, target } => {
                DiagnosticBuilder::new(dcx, level, fluent::errors_target_inconsistent_pointer_width)
                    .arg_mv("pointer_size", pointer_size)
                    .arg_mv("target", target)
            }
            TargetDataLayoutErrors::InvalidBitsSize { err } => {
                DiagnosticBuilder::new(dcx, level, fluent::errors_target_invalid_bits_size)
                    .arg_mv("err", err)
            }
        }
    }
}

/// Utility struct used to apply a single label while highlighting multiple spans
pub struct SingleLabelManySpans {
    pub spans: Vec<Span>,
    pub label: &'static str,
}
impl AddToDiagnostic for SingleLabelManySpans {
    fn add_to_diagnostic_with<F>(self, diag: &mut crate::Diagnostic, _: F) {
        diag.span_labels(self.spans, self.label);
    }
}

#[derive(Subdiagnostic)]
#[label(errors_expected_lifetime_parameter)]
pub struct ExpectedLifetimeParameter {
    #[primary_span]
    pub span: Span,
    pub count: usize,
}

#[derive(Subdiagnostic)]
#[note(errors_delayed_at_with_newline)]
pub struct DelayedAtWithNewline {
    #[primary_span]
    pub span: Span,
    pub emitted_at: DiagnosticLocation,
    pub note: Backtrace,
}
#[derive(Subdiagnostic)]
#[note(errors_delayed_at_without_newline)]
pub struct DelayedAtWithoutNewline {
    #[primary_span]
    pub span: Span,
    pub emitted_at: DiagnosticLocation,
    pub note: Backtrace,
}

impl IntoDiagnosticArg for DiagnosticLocation {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::from(self.to_string()))
    }
}

impl IntoDiagnosticArg for Backtrace {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::from(self.to_string()))
    }
}

#[derive(Subdiagnostic)]
#[note(errors_invalid_flushed_delayed_diagnostic_level)]
pub struct InvalidFlushedDelayedDiagnosticLevel {
    #[primary_span]
    pub span: Span,
    pub level: Level,
}
impl IntoDiagnosticArg for Level {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(Cow::from(self.to_string()))
    }
}

#[derive(Subdiagnostic)]
#[suggestion(errors_indicate_anonymous_lifetime, code = "{suggestion}", style = "verbose")]
pub struct IndicateAnonymousLifetime {
    #[primary_span]
    pub span: Span,
    pub count: usize,
    pub suggestion: String,
}

impl IntoDiagnosticArg for type_ir::ClosureKind {
    fn into_diagnostic_arg(self) -> DiagnosticArgValue<'static> {
        DiagnosticArgValue::Str(self.as_str().into())
    }
}
