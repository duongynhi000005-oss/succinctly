//! A jq-mode interpreter that **carries** jq 1.7.1's path register instead of
//! inferring it (#5223).
//!
//! The resolver in `eval.rs` reconstructs where jq's register would be from the
//! shape of the syntax (`Frame`, `Snapshot`, `FoldRegister`, ...). jq itself
//! keeps the register as three fields of `jq_state` -- `path`,
//! `value_at_path` and `subexp_nest` -- saved into every fork point and
//! restored on every backtrack, and applies one identity rule
//! (`jv_identical`) at three sites: the `INDEX`/`EACH` opcodes, `getpath`'s
//! `_jq_path_append`, and `PATH_END`. This module is that model, written down:
//!
//! - `Reg` is the register. Every output of `Interp::eval` carries its
//!   own, so a generator resumed for its next output continues from the
//!   register it had, which is what jq's fork-point restore amounts to.
//! - Values are [`OwnedValue`]s with jq's identity (`jv_identical`): arrays,
//!   objects, strings and number literals compare by storage, `null`/`true`/
//!   `false` by kind, and computed numbers by their `f64` bits. Every string
//!   and number literal the interpreter holds has storage of its own: the
//!   input document is promoted on entry, a constant once per site, and a C
//!   builtin's result as it is answered (`fresh`), as jq allocates every `jv`.
//! - Each `Expr` arm follows the bytecode `compile.c` emits for it, cited at
//!   the arm. The jq-defined builtins run their `builtin.jq` bodies
//!   ([`PRELUDE`]); the C-coded ones are delegated to the existing evaluator
//!   and answer a fresh value, unless jq's own function hands back its input
//!   or an argument (`c_builtin_identity`), when that value itself is the
//!   answer.
//!
//! It is reachable from `succinctly dev jq-path-mode` (hidden) and from tests,
//! never from `succinctly jq`. A construct it does not implement raises
//! `Raise::Unsupported`, which nothing catches, so a row it cannot run is never
//! mistaken for one it answers. See `docs/plan/jq-path-mode-interpreter-spike.md`
//! for the Phase 0 go/no-go report.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use indexmap::IndexMap;

use super::c_builtin_identity::{array_index, binop_keeps, format_keeps_input, kept, Kept};
use super::error::{dump_truncated_with, Control, EvalError};
use super::eval::{
    apply_compare_op, arith_combine, arith_negate, compare_values, eval_owned_with_file_index,
    owned_to_string, EvalTag, JqSemantics, QueryResult,
};
use super::expr::{
    Builtin, Expr, Literal, NumberKey, ObjectKey, Param, Pattern, SliceBoundKey, StringPart,
};
use super::parser::parse;
use super::value::{
    jv_identical, ArrayVec, NumberRepr, OwnedValue, SharableLiteral, SharableString,
};
use super::walk::{builtin_kids, map_builtin_subexprs, BuiltinKids};

/// A string with storage of its own: a fresh `jv_string`.
fn string(s: &str) -> OwnedValue {
    let mut s = SharableString::from(s);
    s.promote();
    OwnedValue::String(s)
}

/// A number literal with storage of its own: a fresh `jv_number_with_literal`.
fn literal(repr: NumberRepr, spelling: &str) -> OwnedValue {
    let mut spelling = SharableLiteral::from(spelling);
    spelling.promote();
    OwnedValue::NumberLiteral(repr, spelling)
}

/// A C builtin's result as jq's fresh `jv`.
///
/// The existing evaluator may hand back a value sharing storage with what it
/// was given (an `Rc` clone), where jq's builtin built a new `jv`: that would
/// read as identical where jq's is not, so a result identical to one of
/// `inputs` gets storage of its own at the top. Below the top, a container the
/// result shares came from the interpreter's own values, as jq's builtins
/// `jv_copy` their input's elements; everything else the builtin built gets
/// storage ([`promote_built`]). The builtins that *do* return their input
/// unchanged are not answered through here.
fn fresh(value: OwnedValue, inputs: &[&OwnedValue]) -> OwnedValue {
    let mut value = if inputs.iter().any(|i| jv_identical(&value, i)) {
        unshare_top(value)
    } else {
        value
    };
    promote_built(&mut value);
    value
}

/// `value` with new storage at the top, its contents shared.
fn unshare_top(value: OwnedValue) -> OwnedValue {
    match value {
        OwnedValue::Array(items) => OwnedValue::Array(items.iter().cloned().collect()),
        OwnedValue::Object(map) => OwnedValue::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<IndexMap<_, _>>()
                .into(),
        ),
        OwnedValue::String(s) => string(&s),
        OwnedValue::NumberLiteral(repr, spelling) => literal(repr, &spelling),
        other => other,
    }
}

/// Give storage to every string and number literal a builtin built, without
/// descending into a container whose storage is shared: such a container is
/// one of the interpreter's own values, whose scalars already have storage, and
/// writing through it would copy it and lose its identity.
fn promote_built(value: &mut OwnedValue) {
    match value {
        OwnedValue::String(s) => s.promote(),
        OwnedValue::NumberLiteral(_, spelling) => spelling.promote(),
        OwnedValue::Array(items) if !items.is_shared() => {
            items.iter_mut().for_each(promote_built);
        }
        OwnedValue::Object(map) if !map.is_shared() => map.values_mut().for_each(promote_built),
        _ => {}
    }
}

fn truthy(value: &OwnedValue) -> bool {
    !matches!(value, OwnedValue::Null | OwnedValue::Bool(false))
}

/// The address of an array's storage: stable for as long as a handle on it is
/// alive, which the slice-view tables guarantee by holding one.
#[cfg(not(feature = "unshared-containers"))]
fn storage_addr(items: &ArrayVec) -> usize {
    core::ptr::addr_of!(**items) as usize
}

/// A persistent path prefix: a fork shares its parent for free.
#[derive(Clone, Default)]
struct PathList(Option<Rc<(Self, OwnedValue)>>);

impl PathList {
    fn push(&self, key: OwnedValue) -> Self {
        Self(Some(Rc::new((self.clone(), key))))
    }

    fn to_array(&self) -> OwnedValue {
        let mut keys = Vec::new();
        let mut at = self;
        while let Some(node) = &at.0 {
            keys.push(node.1.clone());
            at = &node.0;
        }
        keys.reverse();
        OwnedValue::Array(keys.into())
    }
}

/// jq 1.7.1's register: `path` (`None` is the `jv_null` it holds outside any
/// `path()`, where nothing is tracked), `value_at_path` and `subexp_nest`.
#[derive(Clone)]
struct Reg {
    path: Option<PathList>,
    node: OwnedValue,
    nest: u32,
}

impl Reg {
    fn top() -> Self {
        Self {
            path: None,
            node: OwnedValue::Null,
            nest: 0,
        }
    }

    fn tracking(&self) -> bool {
        self.nest == 0 && self.path.is_some()
    }

    /// `path_intact`: true when not tracking, else `jv_identical`.
    fn intact(&self, value: &OwnedValue) -> bool {
        !self.tracking() || jv_identical(value, &self.node)
    }

    /// `path_append`: a no-op when not tracking.
    fn append(mut self, key: OwnedValue, value: OwnedValue) -> Self {
        if self.tracking() {
            self.path = Some(self.path.as_ref().map(|p| p.push(key)).unwrap_or_default());
            self.node = value;
        }
        self
    }

    /// `SUBEXP_BEGIN`: tracking suspended for the sub-expression.
    fn sub(&self) -> Self {
        let mut reg = self.clone();
        reg.nest += 1;
        reg
    }
}

/// How an evaluation stopped.
#[derive(Debug)]
enum Raise {
    /// A jq error. `catch` sees its payload.
    Error(EvalError),
    /// `break $label`: an error `{"__jq": n}` to `try`, which its own `label`
    /// alone stops.
    Break(u64),
    /// An error raised downstream of a `try` body, wrapped by `TRY_END` so the
    /// `try` re-raises it instead of handling it.
    Wrapped(Box<Self>),
    /// `halt`/`halt_error`: nothing catches it.
    Halt(i32),
    /// A construct the interpreter does not implement: nothing catches it
    /// either.
    Unsupported(String),
}

impl Raise {
    /// Whether `try`, `?//` and `label` read this as a jq error.
    fn is_error(&self) -> bool {
        matches!(self, Self::Error(_) | Self::Break(_))
    }
}

type R = Result<(), Raise>;
type Sink<'s> = dyn FnMut(OwnedValue, Reg) -> R + 's;

/// The label value `GENLABEL` makes: `{"__jq": n}`.
fn label_value(n: u64) -> OwnedValue {
    let mut map = IndexMap::new();
    map.insert(String::from("__jq"), OwnedValue::Int(n as i64));
    OwnedValue::Object(map.into())
}

/// What `catch` receives for a jq error: `error(v)`'s `v` itself, or the
/// message of any other error as a fresh string.
fn caught(raise: Raise) -> OwnedValue {
    match raise {
        Raise::Error(error) => {
            let mut payload = error.payload();
            promote_built(&mut payload);
            payload
        }
        Raise::Break(n) => label_value(n),
        other => unreachable!("only a jq error is caught: {other:?}"), // patchcov: coverage tolerate-line reason="unreachable: every caller checks Raise::is_error first"
    }
}

fn raise_eval(error: EvalError) -> Raise {
    if !error.is_uncatchable() {
        return Raise::Error(error);
    }
    unsupported(format!("uncatchable delegated error: {}", error.message)) // patchcov: coverage tolerate-line reason="defensive: the delegated builtins the interpreter reaches raise only catchable errors; a resource limit would land here"
}

/// An error the interpreter raises itself (jq's three path messages, an object
/// key's type): a plain message, catchable like any jq error. Only
/// [`raise_eval`] reads `EvalError`'s own catchability, which describes the
/// shipped resolver's refusals, not jq.
fn raise_msg(error: EvalError) -> Raise {
    Raise::Error(EvalError::new(error.message))
}

fn unsupported(what: impl Into<String>) -> Raise {
    Raise::Unsupported(what.into())
}

/// The lexical environment: variables, function definitions, closure
/// parameters and labels.
#[derive(Clone, Default)]
struct Env<'e>(Option<Rc<Frame<'e>>>);

enum Frame<'e> {
    Var {
        name: &'e str,
        value: OwnedValue,
        parent: Env<'e>,
    },
    /// A `def`: its body runs in the environment *this frame* closes, so it
    /// sees itself and every definition before it.
    Def {
        name: &'e str,
        params: &'e [Param],
        body: &'e Expr,
        parent: Env<'e>,
    },
    /// A closure parameter: an argument expression and its caller's
    /// environment.
    Arg {
        name: &'e str,
        body: &'e Expr,
        env: Env<'e>,
        parent: Env<'e>,
    },
    Label {
        name: &'e str,
        id: u64,
        parent: Env<'e>,
    },
}

/// A resolved function.
enum Callee<'e> {
    Def {
        params: &'e [Param],
        body: &'e Expr,
        env: Env<'e>,
    },
    Arg {
        body: &'e Expr,
        env: Env<'e>,
    },
}

impl<'e> Env<'e> {
    fn push(frame: Frame<'e>) -> Self {
        Self(Some(Rc::new(frame)))
    }

    fn parent<'a>(frame: &'a Frame<'e>) -> &'a Self {
        match frame {
            Frame::Var { parent, .. }
            | Frame::Def { parent, .. }
            | Frame::Arg { parent, .. }
            | Frame::Label { parent, .. } => parent,
        }
    }

    fn lookup_var(&self, name: &str) -> Option<OwnedValue> {
        let mut at = self;
        while let Some(frame) = &at.0 {
            if let Frame::Var { name: n, value, .. } = &**frame {
                if *n == name {
                    return Some(value.clone());
                }
            }
            at = Self::parent(frame);
        }
        None
    }

    fn lookup_label(&self, name: &str) -> Option<u64> {
        let mut at = self;
        while let Some(frame) = &at.0 {
            if let Frame::Label { name: n, id, .. } = &**frame {
                if *n == name {
                    return Some(*id);
                }
            }
            at = Self::parent(frame);
        }
        None
    }

    fn lookup_func(&self, name: &str, arity: usize) -> Option<Callee<'e>> {
        let mut at = self;
        while let Some(frame) = &at.0 {
            match &**frame {
                Frame::Def {
                    name: n,
                    params,
                    body,
                    ..
                } if *n == name && params.len() == arity => {
                    return Some(Callee::Def {
                        params,
                        body,
                        env: at.clone(),
                    });
                }
                Frame::Arg {
                    name: n, body, env, ..
                } if *n == name && arity == 0 => {
                    return Some(Callee::Arg {
                        body,
                        env: env.clone(),
                    });
                }
                _ => {}
            }
            at = Self::parent(frame);
        }
        None
    }

    /// Bind a variable whose name the program (or prelude) owns.
    fn var_ref(&self, name: &'e str, value: OwnedValue) -> Self {
        Env::push(Frame::Var {
            name,
            value,
            parent: self.clone(),
        })
    }
}

const T_VAR: &str = "__sp_t";

/// The jq-defined builtins the interpreter runs from source.
///
/// This is jq 1.7.1's `src/builtin.jq`, verbatim but for three mechanical
/// changes. Every name
/// carries an `_sp_` prefix, so the parser cannot fold a definition back into
/// the `Builtin` it replaces. Closure parameters are renamed where
/// succinctly's parser would read the jq name as a builtin (`exp`, `paths`).
/// And `_modify`'s private `$$$$v` (`LOADVN`, which only avoids a refcount)
/// is spelled `$v`. `builtin.jq`'s comments are left out.
///
/// `leaf_paths` is a succinctly extension, not a jq builtin: its definition
/// here is the path of every childless node (scalars, `null`, and empty
/// arrays and objects), the root included when it is one, as the shipped
/// evaluator answers it.
///
/// Bodies call one another through the `Builtin` variants they parse to,
/// which `prelude_call` maps back here, plus a few direct `_sp_` calls. The
/// C functions those bodies call that succinctly has no builtin for
/// (`_sort_by_impl`, `_strindices`, `_match_impl`, ...) are `_sp_hook_*`
/// calls, answered by `Interp::hook`.
pub const PRELUDE: &str = r#"
def _sp_select(f): if f then . else empty end;
def _sp_map(f): [.[] | f];
def _sp_recurse(f): def r: ., (f | r); r;
def _sp_recurse(f; cond): def r: ., (f | select(cond) | r); r;
def _sp_recurse: recurse(.[]?);
def _sp_add: reduce .[] as $x (null; . + $x);
def _sp_del(f): delpaths([path(f)]);
def _sp_assign(sp_paths; $value): reduce path(sp_paths) as $p (.; setpath($p; $value));
def _sp_modify(sp_paths; sp_update):
    reduce path(sp_paths) as $p ([., []];
        . as $dot
      | null
      | label $out
      | ($dot[0] | getpath($p)) as $v
      | (
          (   $v
            | sp_update
            | (., break $out) as $v
            | $dot
            | setpath([0] + $p; $v)
          ),
          (
              $dot
            | setpath([1, (.[1] | length)]; $p)
          )
        )
    ) | . as $dot | $dot[0] | delpaths($dot[1]);
def _sp_upd_add(sp_paths; $sp_t): _sp_modify(sp_paths; . + $sp_t);
def _sp_upd_sub(sp_paths; $sp_t): _sp_modify(sp_paths; . - $sp_t);
def _sp_upd_mul(sp_paths; $sp_t): _sp_modify(sp_paths; . * $sp_t);
def _sp_upd_div(sp_paths; $sp_t): _sp_modify(sp_paths; . / $sp_t);
def _sp_upd_mod(sp_paths; $sp_t): _sp_modify(sp_paths; . % $sp_t);
def _sp_upd_alt(sp_paths; $sp_t): _sp_modify(sp_paths; . // $sp_t);
def _sp_map_values(f): .[] |= f;
def _sp_to_entries: [keys_unsorted[] as $k | {key: $k, value: .[$k]}];
def _sp_from_entries: map({(.key // .Key // .name // .Name): (if has("value") then .value else .Value end)}) | add | .//={};
def _sp_with_entries(f): to_entries | map(f) | from_entries;
def _sp_reverse: [.[length - 1 - range(0;length)]];
def _sp_paths: path(..)|select(length > 0);
def _sp_paths(node_filter): path(..|select(node_filter))|select(length > 0);
def _sp_arrays: select(type == "array");
def _sp_objects: select(type == "object");
def _sp_iterables: select(type|. == "array" or . == "object");
def _sp_booleans: select(type == "boolean");
def _sp_numbers: select(type == "number");
def _sp_normals: select(isnormal);
def _sp_finites: select(isfinite);
def _sp_strings: select(type == "string");
def _sp_nulls: select(. == null);
def _sp_values: select(. != null);
def _sp_scalars: select(type|. != "array" and . != "object");
def _sp_join($x): reduce .[] as $i (null;
            (if .==null then "" else .+$x end) +
            ($i | if type=="boolean" or type=="number" then tostring else .//"" end)
        ) // "";
def _sp__flatten($x): reduce .[] as $i ([]; if $i | type == "array" and $x != 0 then . + ($i | _sp__flatten($x-1)) else . + [$i] end);
def _sp_flatten($x): if $x < 0 then error("flatten depth must not be negative") else _sp__flatten($x) end;
def _sp_flatten: _sp__flatten(-1);
def _sp_while(cond; sp_update):
     def _while:
         if cond then ., (sp_update | _while) else empty end;
     _while;
def _sp_until(cond; sp_next):
     def _until:
         if cond then . else (sp_next|_until) end;
     _until;
def _sp_limit($n; sp_exp):
    if $n > 0 then label $out | foreach sp_exp as $item ($n; .-1; $item, if . <= 0 then break $out else empty end)
    elif $n == 0 then empty
    else sp_exp end;
def _sp_range($init; $upto; $by):
    if $by > 0 then $init|while(. < $upto; . + $by)
  elif $by < 0 then $init|while(. > $upto; . + $by)
  else empty end;
def _sp_first(g): label $out | g | ., break $out;
def _sp_isempty(g): first((g|false), true);
def _sp_all(generator; condition): isempty(generator|condition and empty);
def _sp_any(generator; condition): isempty(generator|condition or empty)|not;
def _sp_all(condition): all(.[]; condition);
def _sp_any(condition): any(.[]; condition);
def _sp_all: all(.[]; .);
def _sp_any: any(.[]; .);
def _sp_last(g): reduce g as $item (null; $item);
def _sp_nth($n; g):
  if $n < 0 then error("nth doesn't support negative indices")
  else label $out | foreach g as $item ($n + 1; . - 1; if . <= 0 then $item, break $out else empty end) end;
def _sp_first: .[0];
def _sp_last: .[-1];
def _sp_nth($n): .[$n];
def _sp_in(xs): . as $x | xs | has($x);
def _sp_inside(xs): . as $x | xs | contains($x);
def _sp_repeat(sp_exp):
     def _repeat:
         sp_exp, _repeat;
     _repeat;
def _sp_ascii_downcase:
  explode | map( if 65 <= . and . <= 90 then . + 32  else . end) | implode;
def _sp_ascii_upcase:
  explode | map( if 97 <= . and . <= 122 then . - 32  else . end) | implode;
def _sp_walk(f):
  def w:
    if type == "object"
    then map_values(w)
    elif type == "array" then map(w)
    else .
    end
    | f;
  w;
def _sp_pick(pathexps):
  . as $in
  | reduce path(pathexps) as $a (null;
      setpath($a; $in|getpath($a)) );
def _sp_IN(s): any(s == .; .);
def _sp_IN(src; s): any(src == s; .);
def _sp_abs: if . < 0 then - . else . end;
def _sp_sort_by(f): _sp_hook_sort_by(map([f]));
def _sp_group_by(f): _sp_hook_group_by(map([f]));
def _sp_unique: group_by(.) | map(.[0]);
def _sp_unique_by(f): group_by(f) | map(.[0]);
def _sp_max_by(f): _sp_hook_max_by(map([f]));
def _sp_min_by(f): _sp_hook_min_by(map([f]));
def _sp_indices($i): if type == "array" and ($i|type) == "array" then .[$i]
  elif type == "array" then .[[$i]]
  elif type == "string" and ($i|type) == "string" then _sp_hook_strindices($i)
  else .[$i] end;
def _sp_index($i):   indices($i) | .[0];
def _sp_rindex($i):  indices($i) | .[-1:][0];
def _sp_debug(msg): (msg | debug | empty), .;
def _sp_isfinite: type == "number" and (isinfinite | not);
def _sp_halt_error: halt_error(5);
def _sp_fromdateiso8601: strptime("%Y-%m-%dT%H:%M:%SZ")|mktime;
def _sp_todateiso8601: strftime("%Y-%m-%dT%H:%M:%SZ");
def _sp_fromdate: fromdateiso8601;
def _sp_todate: todateiso8601;
def _sp_match(sp_re; mode): _sp_hook_match_impl(sp_re; mode; false)|.[];
def _sp_match($val): ($val|type) as $vt | if $vt == "string" then match($val; null)
   elif $vt == "array" and ($val | length) > 1 then match($val[0]; $val[1])
   elif $vt == "array" and ($val | length) > 0 then match($val[0]; null)
   else error( $vt + " not a string or array") end;
def _sp_test(sp_re; mode): _sp_hook_match_impl(sp_re; mode; true);
def _sp_test($val): ($val|type) as $vt | if $vt == "string" then test($val; null)
   elif $vt == "array" and ($val | length) > 1 then test($val[0]; $val[1])
   elif $vt == "array" and ($val | length) > 0 then test($val[0]; null)
   else error( $vt + " not a string or array") end;
def _sp_capture(sp_re; mods): match(sp_re; mods) | reduce ( .captures | .[] | select(.name != null) | { (.name) : .string } ) as $pair ({}; . + $pair);
def _sp_capture($val): ($val|type) as $vt | if $vt == "string" then capture($val; null)
   elif $vt == "array" and ($val | length) > 1 then capture($val[0]; $val[1])
   elif $vt == "array" and ($val | length) > 0 then capture($val[0]; null)
   else error( $vt + " not a string or array") end;
def _sp_scan($re; $flags):
  match($re; "g" + $flags)
    | if (.captures|length > 0)
      then [ .captures | .[] | .string ]
      else .string
      end;
def _sp_scan($re): scan($re; null);
def _sp__nwise($n):
  def n: if length <= $n then . else .[0:$n] , (.[$n:] | n) end;
  n;
def _sp_splits($re; flags): . as $s
  | [ match($re; "g" + flags) | (.offset, .offset + .length) ]
  | [0] + . +[$s|length]
  | _sp__nwise(2)
  | $s[.[0]:.[1] ] ;
def _sp_splits($re): splits($re; null);
def _sp_split($re; flags): [ splits($re; flags) ];
def _sp_sub($re; s; $flags):
   . as $in
   | (reduce match($re; $flags) as $edit
        ({result: [], previous: 0};
            $in[ .previous: ($edit | .offset) ] as $gap
            | [reduce ( $edit | .captures | .[] | select(.name != null) | { (.name) : .string } ) as $pair
                 ({}; . + $pair) | s ] as $inserts
            | reduce range(0; $inserts|length) as $ix (.; .result[$ix] += $gap + $inserts[$ix])
            | .previous = ($edit | .offset + .length ) )
          | .result[] + $in[.previous:] )
      // $in;
def _sp_sub($re; s): sub($re; s; "");
def _sp_gsub($re; s; flags): sub($re; s; flags + "g");
def _sp_gsub($re; s): sub($re; s; "g");
def _sp_combinations:
    if length == 0 then [] else
        .[0][] as $x
          | (.[1:] | combinations) as $y
          | [$x] + $y
    end;
def _sp_combinations(n):
    . as $dot
      | [range(n) | $dot]
      | combinations;
def _sp_transpose: [range(0; map(length)|max // 0) as $i | [.[][$i]]];
def _sp_truncate_stream(stream):
  . as $n | null | stream | . as $input | if (.[0]|length) > $n then setpath([0];$input[0][$n:]) else empty end;
def _sp_fromstream(i): {x: null, e: false} as $init |
  foreach i as $i ($init
  ; if .e then $init else . end
  | if $i|length == 2
    then setpath(["e"]; $i[0]|length==0) | setpath(["x"]+$i[0]; $i[1])
    else setpath(["e"]; $i[0]|length==1) end
  ; if .e then .x else empty end);
def _sp_tostream:
  path(def r: (.[]?|r), .; r) as $p |
  getpath($p) |
  reduce path(.[]?) as $q ([$p, .]; [$p+$q]);
def _sp_bsearch($target):
  if length == 0 then -1
  elif length == 1 then
     if $target == .[0] then 0 elif $target < .[0] then -1 else -2 end
  else . as $in
    | [0, length-1, null]
    | until( .[0] > .[1] ;
             if .[2] != null then (.[1] = -1)
             else
               ( ( (.[1] + .[0]) / 2 ) | floor ) as $mid
               | $in[$mid] as $monkey
               | if $monkey == $target  then (.[2] = $mid)
                 elif .[0] == .[1]     then (.[1] = -1)
                 elif $monkey < $target then (.[0] = ($mid + 1))
                 else (.[1] = ($mid - 1))
                 end
             end )
    | if .[2] == null then
         if $in[ .[0] ] < $target then (-2 -.[0])
         else (-1 -.[0])
         end
      else .[2]
      end
  end;
def _sp_INDEX(stream; idx_expr):
  reduce stream as $row ({}; .[$row|idx_expr|tostring] = $row);
def _sp_INDEX(idx_expr): INDEX(.[]; idx_expr);
def _sp_leaf_paths: path(..|select(type != "object" and type != "array" or length == 0));
.
"#;

/// The parsed [`PRELUDE`], plus the few one-off expressions the interpreter
/// needs a stable `&Expr` for.
pub struct Prelude {
    defs: Expr,
    /// `$__sp_t`: a compound assignment's right-hand value, re-read as the
    /// `$sp_t` argument of `_sp_upd_*`.
    t_var: Expr,
    /// `.[]`: the delegated `Cannot iterate over ...` error.
    iterate: Expr,
}

impl Prelude {
    /// Parse the prelude. Panics only if [`PRELUDE`] itself fails to parse,
    /// which a unit test pins.
    pub fn new() -> Self {
        let parse_ok = |src: &str| parse(src).unwrap_or_else(|e| panic!("prelude: {e:?}"));
        Self {
            defs: parse_ok(PRELUDE),
            t_var: parse_ok("$__sp_t"),
            iterate: parse_ok(".[]"),
        }
    }
}

impl Default for Prelude {
    fn default() -> Self {
        Self::new()
    }
}

/// The interpreter: one per program run.
pub struct Interp<'e> {
    prelude: &'e Prelude,
    prelude_env: Env<'e>,
    /// One `jv` per constant site, as `LOADK` hands out (keyed by the `Expr`'s
    /// address, which is stable: the interpreter never rewrites the program).
    consts: RefCell<BTreeMap<usize, OwnedValue>>,
    next_label: Cell<u64>,
    /// Array slices as `jvp_array_slice` makes them: a view (`offset`, `size`)
    /// into the root allocation. Keyed by `(root, offset, size)`, so equal
    /// slices of one array are one value; each view maps back to its root
    /// and offset, so a slice of a slice lands on the same key. Both hold
    /// the root, so a compared address is never reused. Unused under the
    /// `unshared-containers` holdout, which has no storage to view.
    #[cfg_attr(feature = "unshared-containers", allow(dead_code))]
    slices: RefCell<BTreeMap<(usize, usize, usize), ArrayVec>>,
    #[cfg_attr(feature = "unshared-containers", allow(dead_code))]
    slice_roots: RefCell<BTreeMap<usize, (ArrayVec, usize)>>,
}

/// How a run ended.
#[derive(Debug)]
pub enum Outcome {
    /// Every output was produced.
    Done,
    /// An uncaught error.
    Error(EvalError),
    /// `halt`/`halt_error` with this exit code.
    Halt(i32),
    /// A construct the interpreter does not implement.
    Unsupported(String),
}

/// A finished run: the outputs produced before it ended, and how it ended.
#[derive(Debug)]
pub struct Run {
    /// Outputs, in order.
    pub outputs: Vec<OwnedValue>,
    /// How the run ended.
    pub outcome: Outcome,
}

/// Parse `filter` and run it on `input`, a parsed input document. `Err` is a
/// compile error: the filter does not parse, or jq would reject it.
///
/// The document's numbers are literals, as a JSON document's materialize
/// (`NumberLiteral`), and it is moved in, so that promoting its scalars copies
/// nothing.
pub fn run(filter: &str, input: OwnedValue) -> Result<Run, String> {
    let program = parse(filter).map_err(|e| format!("{e:?}"))?;
    // jq's compile-time checks (an unbound `$x`, an undefined function), which
    // the interpreter alone would only meet if evaluation reached them. The
    // resolver rewrites what it checks, so it runs on a copy.
    let diagnostics = super::resolve::resolve_all_jq(&mut program.clone());
    if !diagnostics.is_empty() {
        return Err(format!("{diagnostics:?}"));
    }
    let prelude = Prelude::new();
    let interp = Interp::new(&prelude);
    Ok(interp.run(&program, input))
}

impl<'e> Interp<'e> {
    /// An interpreter over `prelude`'s definitions.
    pub fn new(prelude: &'e Prelude) -> Self {
        let mut env = Env::default();
        let mut at = &prelude.defs;
        while let Expr::FuncDef {
            name,
            params,
            body,
            then,
            ..
        } = at
        {
            env = Env::push(Frame::Def {
                name,
                params,
                body,
                parent: env.clone(),
            });
            at = then;
        }
        Self {
            prelude,
            prelude_env: env,
            consts: RefCell::new(BTreeMap::new()),
            next_label: Cell::new(0),
            slices: RefCell::new(BTreeMap::new()),
            slice_roots: RefCell::new(BTreeMap::new()),
        }
    }

    /// Run `program` on `input`, a document: its strings and numbers are
    /// promoted first, so each node has one identity, as `jv_parse` gives it.
    pub fn run(&self, program: &'e Expr, mut input: OwnedValue) -> Run {
        input.promote_identities();
        let mut outputs = Vec::new();
        let result = self.eval(program, &Env::default(), input, Reg::top(), &mut |v, _| {
            outputs.push(v);
            Ok(())
        });
        let outcome = match result {
            Ok(()) => Outcome::Done,
            Err(raise) => Self::outcome(raise),
        };
        Run { outputs, outcome }
    }

    fn outcome(raise: Raise) -> Outcome {
        match raise {
            Raise::Error(error) => Outcome::Error(error),
            Raise::Break(n) => Outcome::Error(EvalError::from_value(label_value(n))), // patchcov: coverage tolerate-line reason="unreachable: run() rejects a break with no enclosing label with resolve_all_jq, and a label stops its own break"
            Raise::Wrapped(inner) => Self::outcome(*inner), // patchcov: coverage tolerate-line reason="unreachable: a wrapped error is unwrapped by the try or label that wrapped it before it can reach the top"
            Raise::Halt(code) => Outcome::Halt(code),
            Raise::Unsupported(what) => Outcome::Unsupported(what),
        }
    }

    fn eval(
        &self,
        expr: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        match expr {
            Expr::Identity => sink(input, reg),
            Expr::Paren(inner) => self.eval(inner, env, input, reg, sink),
            Expr::Pipe(stages) => self.eval_pipe(stages, env, input, reg, sink),
            // `FORK`: each branch starts from the register at entry.
            Expr::Comma(items) => {
                for item in items {
                    self.eval(item, env, input.clone(), reg.clone(), sink)?;
                }
                Ok(())
            }
            Expr::Literal(lit) => match self.constant(expr) {
                Some(value) => sink(value, reg),
                // A negative number is `_negate` of a literal: computed. The
                // parser folds some negations itself (`-01`, `-1.5`, a
                // negative past 2^53) into a spelling-less `Int`/`Float`.
                None => match lit {
                    Literal::NumberLiteral(NumberRepr::Int(i), _) | Literal::Int(i) => {
                        sink(OwnedValue::Int(*i), reg)
                    }
                    Literal::NumberLiteral(NumberRepr::Float(f), _) | Literal::Float(f) => {
                        sink(OwnedValue::Float(*f), reg)
                    }
                    _ => Err(unsupported("a literal that is not a constant")), // patchcov: coverage tolerate-line reason="unreachable: constant_value answers every literal but a negative number, which the arm above computes"
                },
            },
            Expr::Loc { .. } => {
                let value = self
                    .constant(expr)
                    .ok_or_else(|| unsupported("a location that is not a constant"))?;
                sink(value, reg)
            }
            Expr::Field(name) => self.index(input, reg, string(name), false, sink),
            Expr::Index { idx, key } => {
                let key = index_key(*idx, key.as_ref());
                self.index(input, reg, key, false, sink)
            }
            Expr::Slice {
                start,
                end,
                start_key,
                end_key,
            } => {
                let key = static_slice_key(*start, *end, start_key.as_ref(), end_key.as_ref());
                self.index(input, reg, key, false, sink)
            }
            Expr::Iterate => self.each(input, reg, false, sink),
            Expr::IndexExpr { target, key } => {
                self.index_expr(target, key, env, input, reg, false, sink)
            }
            Expr::SliceExpr { target, start, end } => {
                self.slice_expr(target, start, end, env, input, reg, false, sink)
            }
            Expr::Optional(inner) => self.optional(inner, env, input, reg, sink),
            Expr::Array(inner) => self.collect(expr, inner, env, input, reg, sink),
            Expr::Object(entries) => {
                if let Some(value) = self.constant(expr) {
                    return sink(value, reg);
                }
                let entries: Vec<(&'e ObjectKey, &'e Expr)> =
                    entries.iter().map(|e| (&e.key, &e.value)).collect();
                self.object(&entries, IndexMap::new(), env, &input, &reg, sink)
            }
            Expr::RecursiveDescent => self.call_prelude("_sp_recurse", &[], env, input, reg, sink),
            Expr::Arithmetic {
                op, left, right, ..
            } => {
                if let Some(value) = self.constant(expr) {
                    return sink(value, reg);
                }
                let op = *op;
                self.binop(left, right, env, &input, &reg, sink, &mut |l, r| {
                    if let Some(kept) = binop_keeps(op, l, r) {
                        return Ok(kept.clone());
                    }
                    arith_combine::<JqSemantics>(op, l.clone(), r.clone())
                        .map(|v| fresh(v, &[l, r]))
                })
            }
            Expr::Compare { op, left, right } => {
                if let Some(value) = self.constant(expr) {
                    return sink(value, reg);
                }
                let op = *op;
                self.binop(left, right, env, &input, &reg, sink, &mut |l, r| {
                    Ok(OwnedValue::Bool(apply_compare_op::<JqSemantics>(op, l, r)))
                })
            }
            // `'-' Exp` is `Exp | _negate`: the operand is tracked, not a
            // subexp, and the result is computed.
            Expr::Negate(inner) => self.eval(inner, env, input, reg, &mut |v, r| {
                let negated = arith_negate::<JqSemantics>(v.clone()).map_err(raise_eval)?;
                sink(fresh(negated, &[&v]), r)
            }),
            Expr::And(a, b) => self.and_or(a, b, true, env, input, reg, sink),
            Expr::Or(a, b) => self.and_or(a, b, false, env, input, reg, sink),
            // Bytecoded `not`: a branch on the input, no call.
            Expr::Not => sink(OwnedValue::Bool(!truthy(&input)), reg),
            Expr::Alternative(a, b) => self.defined_or(a, b, env, input, reg, sink),
            // `gen_cond`: the condition is a subexp; each branch starts from
            // the register at entry.
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => self.eval(cond, env, input.clone(), reg.sub(), &mut |c, _| {
                let branch = if truthy(&c) { then_branch } else { else_branch };
                self.eval(branch, env, input.clone(), reg.clone(), sink)
            }),
            Expr::Try { expr: body, catch } => {
                self.try_catch(body, catch.as_deref(), env, input, reg, sink)
            }
            // C `error/1` on the input.
            Expr::Error(None) => Err(Raise::Error(EvalError::from_value(input))),
            // `def error(msg): msg|error;`: `msg` is tracked.
            Expr::Error(Some(msg)) => self.eval(msg, env, input, reg, &mut |v, _| {
                Err(Raise::Error(EvalError::from_value(v)))
            }),
            Expr::Builtin(builtin) => self.builtin(expr, builtin, env, input, reg, sink),
            Expr::StringInterpolation(parts) => self.interpolate(parts, env, &input, &reg, sink),
            // `@text` is `f_tostring`, which returns a string input itself.
            Expr::Format(format) if format_keeps_input(format, &input) => sink(input, reg),
            Expr::Format(_) | Expr::Env => emit_delegated(expr, &input, &[], &reg, sink),
            // `gen_destructure` with a plain `$var`: the source is a subexp,
            // and the body starts from the register at entry.
            Expr::As {
                expr: source,
                var,
                body,
            } => self.eval(source, env, input.clone(), reg.sub(), &mut |v, _| {
                let env = env.var_ref(var, v);
                self.eval(body, &env, input.clone(), reg.clone(), sink)
            }),
            Expr::AsPattern {
                expr: source,
                patterns,
                body,
            } => self.eval(source, env, input.clone(), reg.sub(), &mut |v, _| {
                self.destructure(patterns, v, env, reg.clone(), &mut |env, reg| {
                    self.eval(body, &env, input.clone(), reg, sink)
                })
            }),
            Expr::Var(name) => match env.lookup_var(name) {
                Some(value) => sink(value, reg),
                None => Err(unsupported(format!("${name} is not bound"))), // patchcov: coverage tolerate-line reason="unreachable: run() rejects an unbound variable with resolve_all_jq before evaluating"
            },
            Expr::Reduce {
                input: source,
                patterns,
                init,
                update,
            } => self.reduce(source, patterns, init, update, env, input, reg, sink),
            Expr::Foreach {
                input: source,
                patterns,
                init,
                update,
                extract,
            } => self.foreach(
                source,
                patterns,
                init,
                update,
                extract.as_deref(),
                env,
                input,
                reg,
                sink,
            ),
            Expr::Limit { n, expr: g } => {
                self.call_prelude("_sp_limit", &[n, g], env, input, reg, sink)
            }
            Expr::FirstExpr(g) => self.call_prelude("_sp_first", &[g], env, input, reg, sink),
            Expr::LastExpr(g) => self.call_prelude("_sp_last", &[g], env, input, reg, sink),
            Expr::NthExpr { n, expr: g } => {
                self.call_prelude("_sp_nth", &[n, g], env, input, reg, sink)
            }
            Expr::Until { cond, update } => {
                self.call_prelude("_sp_until", &[cond, update], env, input, reg, sink)
            }
            Expr::While { cond, update } => {
                self.call_prelude("_sp_while", &[cond, update], env, input, reg, sink)
            }
            Expr::Repeat(g) => self.call_prelude("_sp_repeat", &[g], env, input, reg, sink),
            Expr::Range { from, to, step } => match (to, step) {
                (Some(to), None) => self.range(from, to, env, input, reg, sink),
                (Some(to), Some(step)) => {
                    self.call_prelude("_sp_range", &[from, to, step], env, input, reg, sink)
                }
                (None, _) => Err(unsupported("range without an upper bound")),
            },
            Expr::Label { name, body } => self.label(name, body, env, input, reg, sink),
            Expr::Break(name) => match env.lookup_label(name) {
                Some(id) => Err(Raise::Break(id)),
                None => Err(unsupported(format!("$*label-{name} is not defined"))), // patchcov: coverage tolerate-line reason="unreachable: run() rejects a break with no enclosing label with resolve_all_jq before evaluating"
            },
            Expr::FuncDef {
                name,
                params,
                body,
                then,
                ..
            } => {
                let env = Env::push(Frame::Def {
                    name,
                    params,
                    body,
                    parent: env.clone(),
                });
                self.eval(then, &env, input, reg, sink)
            }
            Expr::FuncCall {
                name,
                args,
                builtin_fallback,
            } => match env.lookup_func(name, args.len()) {
                Some(callee) => {
                    let args: Vec<&'e Expr> = args.iter().collect();
                    self.call(callee, &args, env, input, reg, sink)
                }
                None => match builtin_fallback {
                    Some(fallback) => self.eval(fallback, env, input, reg, sink),
                    None => self.hook(name, args, env, input, reg, sink),
                },
            },
            Expr::Assign { path, value } => {
                self.call_prelude("_sp_assign", &[path, value], env, input, reg, sink)
            }
            Expr::Update { path, filter } => {
                self.call_prelude("_sp_modify", &[path, filter], env, input, reg, sink)
            }
            Expr::CompoundAssign { op, path, value } => {
                let name = match op {
                    super::expr::AssignOp::Add => "_sp_upd_add",
                    super::expr::AssignOp::Sub => "_sp_upd_sub",
                    super::expr::AssignOp::Mul(_) => "_sp_upd_mul",
                    super::expr::AssignOp::Div => "_sp_upd_div",
                    super::expr::AssignOp::Mod => "_sp_upd_mod",
                };
                self.update_with(name, path, value, env, input, reg, sink)
            }
            Expr::AlternativeAssign { path, value } => {
                self.update_with("_sp_upd_alt", path, value, env, input, reg, sink)
            }
            other => Err(unsupported(format!("expression {}", variant_name(other)))), // patchcov: coverage tolerate-line reason="defensive: the remaining Expr variants are yq-only or resolver products the jq parser never emits"
        }
    }

    fn eval_pipe(
        &self,
        stages: &'e [Expr],
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        match stages {
            [] => sink(input, reg),
            [only] => self.eval(only, env, input, reg, sink),
            [first, rest @ ..] => self.eval(first, env, input, reg, &mut |v, r| {
                self.eval_pipe(rest, env, v, r, sink)
            }),
        }
    }

    /// `LOADK`: one value per constant site. `None` when `expr` is not
    /// something jq compiles to a single constant.
    fn constant(&self, expr: &'e Expr) -> Option<OwnedValue> {
        let key = expr as *const Expr as usize;
        if let Some(value) = self.consts.borrow().get(&key) {
            return Some(value.clone());
        }
        let value = constant_value(expr)?;
        self.consts.borrow_mut().insert(key, value.clone());
        Some(value)
    }

    /// `INDEX`/`INDEX_OPT` on `container` with `key`: `path_intact` first
    /// (a failure raises even under `INDEX_OPT`), then `jv_get`, then
    /// `path_append` with the key as given. 1.7.1 does not renormalize a
    /// negative index (`[1,2,3] | path(.[-1])` is `[-1]`; 1.8 changed that).
    fn index(
        &self,
        container: OwnedValue,
        reg: Reg,
        key: OwnedValue,
        opt: bool,
        sink: &mut Sink<'_>,
    ) -> R {
        if !reg.intact(&container) {
            return Err(raise_msg(EvalError::invalid_path_expression_near_access(
                &key, &container,
            )));
        }
        match self.jv_get(&container, &key) {
            Ok(value) => {
                let reg = reg.append(key, value.clone());
                sink(value, reg)
            }
            // `INDEX_OPT` drops the get's own error, never an escape the
            // interpreter raises past every handler.
            Err(Raise::Error(_)) if opt => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// `jv_get`, natively for the shapes whose result keeps its identity, and
    /// through the existing evaluator for the rest (array-of-indices keys, and
    /// every error), whose results are fresh.
    fn jv_get(&self, container: &OwnedValue, key: &OwnedValue) -> Result<OwnedValue, Raise> {
        match (container, key) {
            (OwnedValue::Object(map), OwnedValue::String(k)) => {
                Ok(map.get(k.as_str()).cloned().unwrap_or(OwnedValue::Null))
            }
            (OwnedValue::Array(items), k) if k.is_number() => {
                Ok(array_index(items.len(), k).map_or(OwnedValue::Null, |i| items[i].clone()))
            }
            (OwnedValue::Null, OwnedValue::String(_) | OwnedValue::Object(_)) => {
                Ok(OwnedValue::Null)
            }
            (OwnedValue::Null, k) if k.is_number() => Ok(OwnedValue::Null),
            // `jv_string_slice`: by codepoint, and always a fresh string.
            (OwnedValue::String(text), OwnedValue::Object(bounds))
                if slice_bounds(text.chars().count(), bounds).is_some() =>
            {
                let (start, end) = slice_bounds(text.chars().count(), bounds).unwrap_or_default();
                let slice: String = text.chars().skip(start).take(end - start).collect();
                Ok(string(&slice))
            }
            (OwnedValue::Array(items), OwnedValue::Object(bounds))
                if slice_bounds(items.len(), bounds).is_some() =>
            {
                let (start, end) = slice_bounds(items.len(), bounds).unwrap_or_default();
                Ok(self.array_slice(items, start, end))
            }
            _ => {
                let expr = Expr::IndexExpr {
                    target: Box::new(Expr::Identity),
                    key: Box::new(Expr::tracked_value(key.clone())),
                };
                let (outputs, control) = delegate_eval(&expr, container);
                if let Some(control) = control {
                    return Err(control_raise(control));
                }
                outputs
                    .into_iter()
                    .next()
                    .map(|v| fresh(v, &[container, key]))
                    .ok_or_else(|| unsupported("jv_get answered nothing")) // patchcov: coverage tolerate-line reason="defensive: the delegated `.[$k]` answers exactly one value or an error"
            }
        }
    }

    /// `jvp_array_slice`: an empty slice is a fresh `[]`; the whole root is the
    /// root itself; any other range is the one view of that range.
    #[cfg_attr(feature = "unshared-containers", allow(clippy::unused_self))]
    fn array_slice(&self, items: &ArrayVec, start: usize, end: usize) -> OwnedValue {
        if start == end {
            return OwnedValue::Array(ArrayVec::new());
        }
        // The holdout stores an array's elements inline, so there is no
        // storage address to key a view by, and no identity to keep anyway.
        #[cfg(feature = "unshared-containers")]
        return OwnedValue::Array(items[start..end].to_vec().into());
        #[cfg(not(feature = "unshared-containers"))]
        self.array_view(items, start, end)
    }

    /// [`Self::array_slice`]'s non-empty range, as a view of its root.
    #[cfg(not(feature = "unshared-containers"))]
    fn array_view(&self, items: &ArrayVec, start: usize, end: usize) -> OwnedValue {
        let (root, offset) = self
            .slice_roots
            .borrow()
            .get(&storage_addr(items))
            .cloned()
            .unwrap_or_else(|| (items.clone(), 0));
        let (offset, size) = (offset + start, end - start);
        if offset == 0 && size == root.len() {
            return OwnedValue::Array(root);
        }
        let key = (storage_addr(&root), offset, size);
        if let Some(view) = self.slices.borrow().get(&key) {
            return OwnedValue::Array(view.clone());
        }
        let view: ArrayVec = root[offset..offset + size].to_vec().into();
        self.slices.borrow_mut().insert(key, view.clone());
        self.slice_roots
            .borrow_mut()
            .insert(storage_addr(&view), (root, offset));
        OwnedValue::Array(view)
    }

    /// `EACH`/`EACH_OPT`: `path_intact` on the container (raising either
    /// way), then one output per element with the register appended.
    fn each(&self, container: OwnedValue, reg: Reg, opt: bool, sink: &mut Sink<'_>) -> R {
        if !reg.intact(&container) {
            return Err(raise_msg(EvalError::invalid_path_expression_near_iterate(
                &container,
            )));
        }
        match &container {
            OwnedValue::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    let key = OwnedValue::Int(i as i64);
                    sink(item.clone(), reg.clone().append(key, item.clone()))?;
                }
                Ok(())
            }
            OwnedValue::Object(map) => {
                for (k, item) in &**map {
                    sink(item.clone(), reg.clone().append(string(k), item.clone()))?;
                }
                Ok(())
            }
            _ if opt => Ok(()),
            _ => {
                let (_, control) = delegate_eval(&self.prelude.iterate, &container);
                Err(control.map_or_else(|| unsupported("iterate answered"), control_raise))
            }
        }
    }

    /// `gen_index`: the key is a subexp evaluated (outermost) on this node's
    /// input, then the target runs on the same input from the entry register.
    #[allow(clippy::too_many_arguments)]
    fn index_expr(
        &self,
        target: &'e Expr,
        key: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        opt: bool,
        sink: &mut Sink<'_>,
    ) -> R {
        self.eval(key, env, input.clone(), reg.sub(), &mut |k, _| {
            self.eval(target, env, input.clone(), reg.clone(), &mut |t, rt| {
                self.index(t, rt, k.clone(), opt, sink)
            })
        })
    }

    /// `gen_slice_index`: the `{"start":..,"end":..}` key is built from two
    /// subexps (start outer), then the target, then `INDEX`.
    #[allow(clippy::too_many_arguments)]
    fn slice_expr(
        &self,
        target: &'e Expr,
        start: &'e Option<Box<Expr>>,
        end: &'e Option<Box<Expr>>,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        opt: bool,
        sink: &mut Sink<'_>,
    ) -> R {
        self.bound(start.as_deref(), env, &input, &reg, &mut |s| {
            self.bound(end.as_deref(), env, &input, &reg, &mut |e| {
                let key = slice_key(Some(s.clone()), Some(e));
                self.eval(target, env, input.clone(), reg.clone(), &mut |t, rt| {
                    self.index(t, rt, key.clone(), opt, sink)
                })
            })
        })
    }

    fn bound(
        &self,
        bound: Option<&'e Expr>,
        env: &Env<'e>,
        input: &OwnedValue,
        reg: &Reg,
        k: &mut dyn FnMut(OwnedValue) -> R,
    ) -> R {
        match bound {
            None => k(OwnedValue::Null),
            Some(e) => self.eval(e, env, input.clone(), reg.sub(), &mut |v, _| k(v)),
        }
    }

    /// `Term '?'` forms: `.k?`, `.[k]?`, `.[]?` and slices compile to the
    /// `_OPT` opcodes, which suppress the get's own error but still raise a
    /// `path_intact` failure. Anything else is `try E`.
    fn optional(
        &self,
        inner: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        match inner {
            Expr::Field(name) => self.index(input, reg, string(name), true, sink),
            Expr::Index { idx, key } => {
                let key = index_key(*idx, key.as_ref());
                self.index(input, reg, key, true, sink)
            }
            Expr::Slice {
                start,
                end,
                start_key,
                end_key,
            } => {
                let key = static_slice_key(*start, *end, start_key.as_ref(), end_key.as_ref());
                self.index(input, reg, key, true, sink)
            }
            Expr::Iterate => self.each(input, reg, true, sink),
            Expr::IndexExpr { target, key } => {
                self.index_expr(target, key, env, input, reg, true, sink)
            }
            Expr::SliceExpr { target, start, end } => {
                self.slice_expr(target, start, end, env, input, reg, true, sink)
            }
            _ => self.try_catch(inner, None, env, input, reg, sink),
        }
    }

    /// `gen_collect`: a constant array is one `LOADK`; otherwise `E` runs
    /// tracked from the entry register, and the array carries the register
    /// at entry (the `FORK` restores it).
    fn collect(
        &self,
        node: &'e Expr,
        inner: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        if let Some(value) = self.constant(node) {
            return sink(value, reg);
        }
        let mut items = Vec::new();
        self.eval(inner, env, input, reg.clone(), &mut |v, _| {
            items.push(v);
            Ok(())
        })?;
        sink(OwnedValue::Array(items.into()), reg)
    }

    /// `gen_dictpair` per entry: key then value, both subexps, earlier
    /// entries outermost; the object is fresh and carries the entry register.
    fn object(
        &self,
        entries: &[(&'e ObjectKey, &'e Expr)],
        built: IndexMap<String, OwnedValue>,
        env: &Env<'e>,
        input: &OwnedValue,
        reg: &Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let Some(((key, value), rest)) = entries.split_first() else {
            return sink(OwnedValue::Object(built.into()), reg.clone());
        };
        let mut with_key = |k: OwnedValue| -> R {
            let OwnedValue::String(name) = &k else {
                return Err(raise_msg(EvalError::cannot_use_as_object_key(&k)));
            };
            self.eval(value, env, input.clone(), reg.sub(), &mut |v, _| {
                let mut next = built.clone();
                next.insert(name.as_str().to_string(), v);
                self.object(rest, next, env, input, reg, sink)
            })
        };
        match key {
            ObjectKey::Literal(s) => with_key(string(s)),
            ObjectKey::Expr(k) => {
                self.eval(k, env, input.clone(), reg.sub(), &mut |kv, _| with_key(kv))
            }
            ObjectKey::Bare => Err(unsupported("a bare object entry (yq only)")), // patchcov: coverage tolerate-line reason="unreachable: a bare object entry is yq syntax, never produced by the jq parser"
        }
    }

    /// A C binary builtin (`_plus`, `_equal`, ...): both operands are
    /// subexps, the right one the outer loop (`[(1,2)+(10,20)]` is
    /// `[11,12,21,22]`); the result keeps the entry register.
    #[allow(clippy::too_many_arguments)]
    fn binop(
        &self,
        left: &'e Expr,
        right: &'e Expr,
        env: &Env<'e>,
        input: &OwnedValue,
        reg: &Reg,
        sink: &mut Sink<'_>,
        op: &mut dyn FnMut(&OwnedValue, &OwnedValue) -> Result<OwnedValue, EvalError>,
    ) -> R {
        self.eval(right, env, input.clone(), reg.sub(), &mut |r, _| {
            self.eval(left, env, input.clone(), reg.sub(), &mut |l, _| {
                let value = op(&l, &r).map_err(raise_eval)?;
                sink(value, reg.clone())
            })
        })
    }

    /// `gen_and`/`gen_or`: `A` is tracked from the entry register; `B` runs
    /// on the duplicated input with the register `A` left; the bool carries
    /// the register of whichever side decided it.
    #[allow(clippy::too_many_arguments)]
    fn and_or(
        &self,
        a: &'e Expr,
        b: &'e Expr,
        is_and: bool,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        self.eval(a, env, input.clone(), reg, &mut |av, ar| {
            if truthy(&av) != is_and {
                return sink(OwnedValue::Bool(!is_and), ar);
            }
            self.eval(b, env, input.clone(), ar, &mut |bv, br| {
                sink(OwnedValue::Bool(truthy(&bv)), br)
            })
        })
    }

    /// `gen_definedor`: `A`'s truthy outputs carry `A`'s register; if none
    /// was found, `B` starts from the entry register. An error in `A`
    /// propagates (`ON_BACKTRACK(FORK)` re-raises).
    fn defined_or(
        &self,
        a: &'e Expr,
        b: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let mut found = false;
        self.eval(a, env, input.clone(), reg.clone(), &mut |v, r| {
            if truthy(&v) {
                found = true;
                sink(v, r)
            } else {
                Ok(())
            }
        })?;
        if found {
            return Ok(());
        }
        self.eval(b, env, input, reg, sink)
    }

    /// `gen_try` (1.7.1's `TRY_BEGIN`/`TRY_END`): an error raised in the body
    /// runs the handler on its payload at the register of `try` entry; an
    /// error raised downstream of a body output passes through wrapped, and
    /// the `try` unwraps it once and re-raises it.
    fn try_catch(
        &self,
        body: &'e Expr,
        catch: Option<&'e Expr>,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let result = self.eval(body, env, input, reg.clone(), &mut |v, r| {
            sink(v, r).map_err(|e| Raise::Wrapped(Box::new(e)))
        });
        match result {
            Ok(()) => Ok(()),
            Err(Raise::Wrapped(inner)) => Err(*inner),
            Err(raise) if raise.is_error() => match catch {
                None => Ok(()),
                Some(handler) => self.eval(handler, env, caught(raise), reg, sink),
            },
            Err(other) => Err(other),
        }
    }

    /// `gen_label`: `try E catch (if . == $label then empty else error)`, the
    /// label a fresh `{"__jq": n}` (`GENLABEL`).
    fn label(
        &self,
        name: &'e str,
        body: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let id = self.next_label.get();
        self.next_label.set(id + 1);
        let env = Env::push(Frame::Label {
            name,
            id,
            parent: env.clone(),
        });
        let result = self.eval(body, &env, input, reg, &mut |v, r| {
            sink(v, r).map_err(|e| Raise::Wrapped(Box::new(e)))
        });
        match result {
            Ok(()) => Ok(()),
            Err(Raise::Wrapped(inner)) => Err(*inner),
            Err(Raise::Break(n)) if n == id => Ok(()),
            // `error({"__jq": n})` raised by hand is this label's break too.
            Err(Raise::Error(e)) if is_label_payload(&e, id) => Ok(()),
            Err(other) => Err(other),
        }
    }

    /// `bind_alternation_matchers`: one pattern binds directly. With `?//`,
    /// every variable any alternative names starts `null`; each alternative
    /// but the last is a `DESTRUCTURE_ALT` fork, which on *any* error raised
    /// after it -- in the pattern, the body, or further downstream -- tries
    /// the next alternative from the register the source left.
    fn destructure(
        &self,
        patterns: &'e [Pattern],
        value: OwnedValue,
        env: &Env<'e>,
        reg: Reg,
        k: &mut dyn FnMut(Env<'e>, Reg) -> R,
    ) -> R {
        if let [only] = patterns {
            return self.match_pattern(only, value, env.clone(), reg, k);
        }
        let mut names = Vec::new();
        for p in patterns {
            pattern_vars(p, &mut names);
        }
        let mut base = env.clone();
        for name in &names {
            base = base.var_ref(name, OwnedValue::Null);
        }
        let (last, alternatives) = patterns.split_last().expect("at least one pattern");
        for p in alternatives {
            match self.match_pattern(p, value.clone(), base.clone(), reg.clone(), k) {
                Ok(()) => return Ok(()),
                Err(raise) if raise.is_error() || matches!(raise, Raise::Wrapped(_)) => continue,
                Err(other) => return Err(other),
            }
        }
        self.match_pattern(last, value, base, reg, k)
    }

    /// One pattern: `$v` is a `STOREV`; `[p0, ..]` is `gen_array_matcher`,
    /// whose `INDEX`es run **last element first** on the bound value; and
    /// `{k: p, ..}` is `gen_object_matcher` per entry in order, its key a
    /// subexp of the bound value. Every `INDEX` checks and moves the register.
    fn match_pattern(
        &self,
        pattern: &'e Pattern,
        value: OwnedValue,
        env: Env<'e>,
        reg: Reg,
        k: &mut dyn FnMut(Env<'e>, Reg) -> R,
    ) -> R {
        match pattern {
            Pattern::Var(name) => k(env.var_ref(name, value), reg),
            Pattern::Array(elems) => self.match_array(elems, elems.len(), value, env, reg, k),
            Pattern::Object(entries) => self.match_object(entries, value, env, reg, k),
        }
    }

    fn match_array(
        &self,
        elems: &'e [Pattern],
        remaining: usize,
        value: OwnedValue,
        env: Env<'e>,
        reg: Reg,
        k: &mut dyn FnMut(Env<'e>, Reg) -> R,
    ) -> R {
        let Some(i) = remaining.checked_sub(1) else {
            return k(env, reg);
        };
        let key = OwnedValue::Int(i as i64);
        self.index(value.clone(), reg, key, false, &mut |element, r| {
            self.match_pattern(&elems[i], element, env.clone(), r, &mut |env, r| {
                self.match_array(elems, i, value.clone(), env, r, k)
            })
        })
    }

    fn match_object(
        &self,
        entries: &'e [super::expr::PatternEntry],
        value: OwnedValue,
        env: Env<'e>,
        reg: Reg,
        k: &mut dyn FnMut(Env<'e>, Reg) -> R,
    ) -> R {
        let Some((entry, rest)) = entries.split_first() else {
            return k(env, reg);
        };
        let mut with_key = |key: OwnedValue| -> R {
            self.index(value.clone(), reg.clone(), key, false, &mut |field, r| {
                let env = match &entry.bind {
                    Some(name) => env.var_ref(name, field.clone()),
                    None => env.clone(),
                };
                self.match_pattern(&entry.pattern, field, env, r, &mut |env, r| {
                    self.match_object(rest, value.clone(), env, r, k)
                })
            })
        };
        match &entry.key {
            ObjectKey::Literal(s) => with_key(string(s)),
            ObjectKey::Expr(e) => {
                self.eval(e, &env, value.clone(), reg.sub(), &mut |kv, _| with_key(kv))
            }
            ObjectKey::Bare => Err(unsupported("a bare pattern key")), // patchcov: coverage tolerate-line reason="unreachable: a bare pattern key is yq syntax, never produced by the jq parser"
        }
    }

    /// `gen_reduce`: `DUP; INIT; STOREV; FORK [DUPN; SOURCE; P; LOADVN;
    /// UPDATE; STOREV; BACKTRACK]; LOADVN`. Each step's `SOURCE` runs on the
    /// fold's input from the register `INIT` left; `UPDATE` starts at the
    /// register the source emission and pattern left, and its movement is
    /// undone; the state is the last `UPDATE` output, or `null` if there was
    /// none (`LOADVN`); the result carries the register after `INIT`. Only
    /// the first `INIT` output's `SOURCE` sees the input ([`FoldInput`]).
    #[allow(clippy::too_many_arguments)]
    fn reduce(
        &self,
        source: &'e Expr,
        patterns: &'e [Pattern],
        init: &'e Expr,
        update: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let fold_input = FoldInput::new(input.clone());
        self.eval(init, env, input, reg, &mut |initial, reg_init| {
            let state = RefCell::new(initial);
            self.eval(
                source,
                env,
                fold_input.take(),
                reg_init.clone(),
                &mut |s, reg_s| {
                    self.destructure(patterns, s, env, reg_s, &mut |env_p, reg_p| {
                        let current = state.replace(OwnedValue::Null);
                        self.eval(update, &env_p, current, reg_p, &mut |u, _| {
                            *state.borrow_mut() = u;
                            Ok(())
                        })
                    })
                },
            )?;
            sink(state.into_inner(), reg_init)
        })
    }

    /// `gen_foreach`: as `reduce`, but `EXTRACT` runs straight after each
    /// `UPDATE` output, on that output and at the register `UPDATE` left.
    #[allow(clippy::too_many_arguments)]
    fn foreach(
        &self,
        source: &'e Expr,
        patterns: &'e [Pattern],
        init: &'e Expr,
        update: &'e Expr,
        extract: Option<&'e Expr>,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let fold_input = FoldInput::new(input.clone());
        self.eval(init, env, input, reg, &mut |initial, reg_init| {
            let state = RefCell::new(initial);
            self.eval(source, env, fold_input.take(), reg_init, &mut |s, reg_s| {
                self.destructure(patterns, s, env, reg_s, &mut |env_p, reg_p| {
                    let current = state.replace(OwnedValue::Null);
                    self.eval(update, &env_p, current, reg_p, &mut |u, reg_u| {
                        *state.borrow_mut() = u.clone();
                        match extract {
                            None => sink(u, reg_u),
                            Some(x) => self.eval(x, &env_p, u, reg_u, sink),
                        }
                    })
                })
            })
        })
    }

    /// `range/2` (bytecoded): `start` and `end` are closure calls, so both
    /// are tracked, `end` from the register `start` left; `RANGE` emits the
    /// start value itself first, then computed successors.
    fn range(
        &self,
        from: &'e Expr,
        to: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        self.eval(from, env, input.clone(), reg, &mut |start, reg_s| {
            self.eval(to, env, input.clone(), reg_s, &mut |end, reg_e| {
                let (Some(mut cur), Some(max)) = (number_f64(&start), number_f64(&end)) else {
                    return Err(Raise::Error(EvalError::new("Range bounds must be numeric")));
                };
                let mut first = Some(start.clone());
                // `RANGE` stops only on `>=`, which a NaN bound never is.
                loop {
                    if cur >= max {
                        return Ok(());
                    }
                    let value = first.take().unwrap_or_else(|| number_owned(cur));
                    sink(value, reg_e.clone())?;
                    cur += 1.0;
                }
            })
        })
    }

    /// `PATH_BEGIN … PATH_END`: a fresh register on the input; each output
    /// must be identical to where the register ended, and yields the path
    /// with the outer register restored.
    fn path_mode(
        &self,
        f: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let inner = Reg {
            path: Some(PathList::default()),
            node: input.clone(),
            nest: 0,
        };
        self.eval(f, env, input, inner, &mut |v, r| {
            if !r.intact(&v) {
                return Err(raise_msg(EvalError::invalid_path_expression(&v)));
            }
            let path = r.path.as_ref().map_or(OwnedValue::Null, PathList::to_array);
            sink(path, reg.clone())
        })
    }

    /// `getpath(p)`, the one register-aware C builtin: `p` is a subexp; on an
    /// input identical to the register's node the path is extended and the
    /// register moves to the result (`_jq_path_append`); otherwise the value
    /// is returned and the register is untouched.
    fn getpath(
        &self,
        p: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        self.eval(p, env, input.clone(), reg.sub(), &mut |path, _| {
            let OwnedValue::Array(keys) = &path else {
                return Err(raise_msg(EvalError::path_must_be_array()));
            };
            let mut value = input.clone();
            for key in &**keys {
                value = self.jv_get(&value, key)?;
            }
            if !reg.tracking() || !jv_identical(&input, &reg.node) {
                return sink(value, reg.clone());
            }
            let mut moved = reg.clone();
            let mut list = moved.path.take().unwrap_or_default();
            for key in &**keys {
                list = list.push(key.clone());
            }
            moved.path = Some(list);
            moved.node = value.clone();
            sink(value, moved)
        })
    }

    fn builtin(
        &self,
        node: &'e Expr,
        builtin: &'e Builtin,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        match builtin {
            Builtin::Path(f) => self.path_mode(f, env, input, reg, sink),
            Builtin::GetPath(p) => self.getpath(p, env, input, reg, sink),
            Builtin::Empty => Ok(()),
            // `f_min`/`f_max` (`minmax_by(x, x)`) and `f_sort` (`jv_sort(x, x)`)
            // keep the input's elements; anything but an array is the shipped
            // builtin's error.
            Builtin::Min | Builtin::Max if matches!(input, OwnedValue::Array(_)) => {
                let is_min = matches!(builtin, Builtin::Min);
                sink(min_max_by(&input, &input, is_min)?, reg)
            }
            Builtin::Sort if matches!(input, OwnedValue::Array(_)) => {
                sink(sort_by_keys(&input, &input, false)?, reg)
            }
            _ => {
                if let Some((name, args)) = prelude_call(builtin) {
                    return self.call_prelude(name, &args, env, input, reg, sink);
                }
                if let Some(why) = unsupported_builtin(builtin) {
                    return Err(unsupported(why));
                }
                self.c_builtin(node, builtin, env, input, reg, sink)
            }
        }
    }

    /// A C builtin: its arguments are subexps the interpreter evaluates itself
    /// (so a closure parameter in one resolves here), then the call is
    /// delegated with the argument values in place. It answers fresh, unless
    /// the builtin is one that hands back its input or an argument
    /// ([`kept`]), when that value itself is the answer.
    fn c_builtin(
        &self,
        node: &'e Expr,
        builtin: &'e Builtin,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let kids = builtin_args(builtin);
        if kids.is_empty() {
            return emit_c_builtin(node, builtin, &input, &[], &reg, sink);
        }
        self.c_args(&kids, env, &input, &reg, &mut |values| {
            let mut next = values.iter();
            let call = map_builtin_subexprs(builtin, &mut |_| {
                Expr::tracked_value(next.next().cloned().unwrap_or(OwnedValue::Null))
            });
            emit_c_builtin(&Expr::Builtin(call), builtin, &input, values, &reg, sink)
        })
    }

    /// The arguments of a C function call (`expand_call_arglist`): each a
    /// subexp on the call's input, the **last** the outermost loop
    /// (`[pow((1,2);(3,4))]` is `[1,8,1,16]` in jq 1.7.1). `k` gets the
    /// values in argument order.
    fn c_args(
        &self,
        args: &[&'e Expr],
        env: &Env<'e>,
        input: &OwnedValue,
        reg: &Reg,
        k: &mut dyn FnMut(&[OwnedValue]) -> R,
    ) -> R {
        let Some((last, init)) = args.split_last() else {
            return k(&[]);
        };
        self.eval(last, env, input.clone(), reg.sub(), &mut |v, _| {
            self.c_args(init, env, input, reg, &mut |values| {
                let mut all = values.to_vec();
                all.push(v.clone());
                k(&all)
            })
        })
    }

    /// String interpolation, `"a\(E)b"`: jq's parser builds it from
    /// `format("text")` (or the prefix's format, which the parser already
    /// put in each slot) and `_plus`, so each slot is a C argument, the last
    /// slot the outermost loop, and the result is a fresh string.
    fn interpolate(
        &self,
        parts: &'e [StringPart],
        env: &Env<'e>,
        input: &OwnedValue,
        reg: &Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let slots: Vec<&'e Expr> = parts
            .iter()
            .filter_map(|part| match part {
                StringPart::Expr(e) => Some(&**e),
                StringPart::Literal(_) => None,
            })
            .collect();
        self.c_args(&slots, env, input, reg, &mut |values| {
            let mut text = String::new();
            let mut next = values.iter();
            for part in parts {
                match part {
                    StringPart::Literal(s) => text.push_str(s),
                    StringPart::Expr(_) => {
                        if let Some(v) = next.next() {
                            text.push_str(&owned_to_string::<JqSemantics>(v));
                        }
                    }
                }
            }
            sink(string(&text), reg.clone())
        })
    }

    /// The C functions the prelude's `builtin.jq` bodies call that succinctly
    /// has no builtin for. Each argument is a subexp; the sort, group and
    /// min/max helpers keep the input's elements (`jv_copy`), so their
    /// identity survives as it does in jq.
    fn hook(
        &self,
        name: &str,
        args: &'e [Expr],
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let undefined = || unsupported(format!("{name}/{} is not defined", args.len()));
        if name == "_sp_hook_match_impl" {
            return self.match_impl(args, env, input, reg, sink);
        }
        let [arg] = args else {
            return Err(undefined());
        };
        let hook = match name {
            "_sp_hook_sort_by" => Hook::SortBy,
            "_sp_hook_group_by" => Hook::GroupBy,
            "_sp_hook_min_by" => Hook::MinBy,
            "_sp_hook_max_by" => Hook::MaxBy,
            "_sp_hook_strindices" => Hook::StrIndices,
            _ => return Err(undefined()),
        };
        self.eval(arg, env, input.clone(), reg.sub(), &mut |a, _| match hook {
            Hook::SortBy => sink(sort_by_keys(&input, &a, false)?, reg.clone()),
            Hook::GroupBy => sink(sort_by_keys(&input, &a, true)?, reg.clone()),
            Hook::MinBy => sink(min_max_by(&input, &a, true)?, reg.clone()),
            Hook::MaxBy => sink(min_max_by(&input, &a, false)?, reg.clone()),
            Hook::StrIndices => {
                let call =
                    Expr::Builtin(Builtin::Indices(Box::new(Expr::tracked_value(a.clone()))));
                emit_delegated(&call, &input, &[&a], &reg, sink)
            }
        })
    }

    /// `_match_impl(re; mode; test)`: a C function, so its three arguments
    /// are subexps (the last outermost). `test` answers `test(re; mode)`'s
    /// boolean; otherwise the array of every match, which `match/2` iterates.
    fn match_impl(
        &self,
        args: &'e [Expr],
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let kids: Vec<&'e Expr> = args.iter().collect();
        self.c_args(&kids, env, &input, &reg, &mut |values| {
            let [re, mode, test] = values else {
                return Err(unsupported(format!("_match_impl/{}", values.len())));
                // patchcov: coverage tolerate-line reason="unreachable: the prelude calls the hook with three arguments only"
            };
            let (re_arg, mode_arg) = (
                Box::new(Expr::tracked_value(re.clone())),
                Box::new(Expr::tracked_value(mode.clone())),
            );
            let call = if truthy(test) {
                Expr::Builtin(Builtin::TestFlags(re_arg, mode_arg))
            } else {
                Expr::Array(Box::new(Expr::Builtin(Builtin::MatchFlags(
                    re_arg, mode_arg,
                ))))
            };
            emit_delegated(&call, &input, &[re, mode], &reg, sink)
        })
    }

    fn call_prelude(
        &self,
        name: &str,
        args: &[&'e Expr],
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let callee = self
            .prelude_env
            .lookup_func(name, args.len())
            .ok_or_else(|| unsupported(format!("prelude {name}/{}", args.len())))?;
        self.call(callee, args, env, input, reg, sink)
    }

    /// `CALL_JQ`: closure parameters bind the argument expressions with the
    /// caller's environment; a `$name` parameter is jq's `name as $name`
    /// desugaring, its argument a subexp, earlier parameters outermost.
    fn call(
        &self,
        callee: Callee<'e>,
        args: &[&'e Expr],
        caller: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let args: Vec<(&'e Expr, Env<'e>)> = args.iter().map(|a| (*a, caller.clone())).collect();
        self.call_with(callee, &args, input, reg, sink)
    }

    /// [`Self::call`] with each argument closing over its own environment.
    fn call_with(
        &self,
        callee: Callee<'e>,
        args: &[(&'e Expr, Env<'e>)],
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        match callee {
            Callee::Arg { body, env } => self.eval(body, &env, input, reg, sink),
            Callee::Def { params, body, env } => {
                let mut body_env = env;
                for (param, (arg, arg_env)) in params.iter().zip(args) {
                    body_env = Env::push(Frame::Arg {
                        name: param.name(),
                        body: arg,
                        env: arg_env.clone(),
                        parent: body_env.clone(),
                    });
                }
                self.bind_dollars(params, body, body_env, input, reg, sink)
            }
        }
    }

    fn bind_dollars(
        &self,
        params: &'e [Param],
        body: &'e Expr,
        env: Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        let Some((param, rest)) = params.split_first() else {
            return self.eval(body, &env, input, reg, sink);
        };
        let Param::Dollar(name) = param else {
            return self.bind_dollars(rest, body, env, input, reg, sink);
        };
        let callee = env
            .lookup_func(name, 0)
            .ok_or_else(|| unsupported(format!("${name} parameter")))?;
        self.call(callee, &[], &env, input.clone(), reg.sub(), &mut |v, _| {
            let env = env.var_ref(name, v);
            self.bind_dollars(rest, body, env, input.clone(), reg.clone(), sink)
        })
    }

    /// `gen_update`/`gen_definedor_assign`: `DUP; VALUE; STOREV tmp;
    /// _modify(path; . op $tmp)`. `VALUE` is tracked (not a subexp), so its
    /// register effect carries into the `_modify` call.
    #[allow(clippy::too_many_arguments)]
    fn update_with(
        &self,
        name: &str,
        path: &'e Expr,
        value: &'e Expr,
        env: &Env<'e>,
        input: OwnedValue,
        reg: Reg,
        sink: &mut Sink<'_>,
    ) -> R {
        self.eval(value, env, input.clone(), reg, &mut |t, reg_t| {
            // `path` closes over the caller's environment; the re-read
            // `$__sp_t` over one that binds the value just computed.
            let env_t = env.var_ref(T_VAR, t);
            let callee = self
                .prelude_env
                .lookup_func(name, 2)
                .ok_or_else(|| unsupported(format!("prelude {name}/2")))?;
            let args = [(path, env.clone()), (&self.prelude.t_var, env_t)];
            self.call_with(callee, &args, input.clone(), reg_t, sink)
        })
    }
}

/// Whether `error` is `error({"__jq": id})`: the error a `break $label` is.
fn is_label_payload(error: &EvalError, id: u64) -> bool {
    let super::error::EvalErrorPayload::Value(OwnedValue::Object(map)) = &error.value else {
        return false;
    };
    map.len() == 1
        && map
            .get("__jq")
            .and_then(OwnedValue::as_f64)
            .is_some_and(|n| n == id as f64)
}

/// The fold's input as `DUPN` leaves it: `gen_reduce`/`gen_foreach` start
/// each loop with `DUPN`, which *moves* the input out of a stack slot that
/// `INIT`'s fork point shares, so every `INIT` output after the first runs
/// `SOURCE` on `null` (`{"a":1} | [reduce .a as $i ((1,2); [$i,.])]` is
/// `[[1,1],[null,2]]` in jq 1.7.1, which value mode already reproduces).
struct FoldInput(RefCell<Option<OwnedValue>>);

impl FoldInput {
    fn new(input: OwnedValue) -> Self {
        Self(RefCell::new(Some(input)))
    }

    fn take(&self) -> OwnedValue {
        self.0.borrow_mut().take().unwrap_or(OwnedValue::Null)
    }
}

/// The prelude's C helpers ([`Interp::hook`]).
#[derive(Clone, Copy)]
enum Hook {
    SortBy,
    GroupBy,
    MinBy,
    MaxBy,
    StrIndices,
}

/// `type_error2`: `"<kind> (<dump>) and <kind> (<dump>) <msg>"`.
fn type_error2(a: &OwnedValue, b: &OwnedValue, msg: &str) -> Raise {
    Raise::Error(EvalError::new(format!(
        "{} ({}) and {} ({}) {msg}",
        a.type_name(),
        dump_truncated_with(EvalTag::Jq, a),
        b.type_name(),
        dump_truncated_with(EvalTag::Jq, b)
    )))
}

/// `_sort_by_impl`/`_group_by_impl`: a stable sort of the input's elements by
/// `keys` (`jv_sort`/`jv_group`), grouping runs of equal keys when `group`.
fn sort_by_keys(input: &OwnedValue, keys: &OwnedValue, group: bool) -> Result<OwnedValue, Raise> {
    let (OwnedValue::Array(items), OwnedValue::Array(ks)) = (input, keys) else {
        return Err(type_error2(
            input,
            keys,
            "cannot be sorted, as they are not both arrays",
        ));
    };
    if items.len() != ks.len() {
        return Err(type_error2(
            input,
            keys,
            "cannot be sorted, as they are not both arrays",
        ));
    }
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| compare_values::<JqSemantics>(&ks[a], &ks[b]));
    if !group {
        return Ok(OwnedValue::Array(
            order.iter().map(|&i| items[i].clone()).collect(),
        ));
    }
    let mut groups: Vec<OwnedValue> = Vec::new();
    let mut current: Vec<OwnedValue> = Vec::new();
    let mut key: Option<usize> = None;
    for &i in &order {
        if let Some(k) = key {
            if compare_values::<JqSemantics>(&ks[k], &ks[i]) != core::cmp::Ordering::Equal {
                groups.push(OwnedValue::Array(core::mem::take(&mut current).into()));
                key = Some(i);
            }
        } else {
            key = Some(i);
        }
        current.push(items[i].clone());
    }
    if key.is_some() {
        groups.push(OwnedValue::Array(current.into()));
    }
    Ok(OwnedValue::Array(groups.into()))
}

/// `minmax_by`: the first element with the least key, or the last with the
/// greatest; `null` for an empty array.
fn min_max_by(values: &OwnedValue, keys: &OwnedValue, is_min: bool) -> Result<OwnedValue, Raise> {
    let (OwnedValue::Array(items), OwnedValue::Array(ks)) = (values, keys) else {
        return Err(type_error2(values, keys, "cannot be iterated over"));
    };
    if items.len() != ks.len() {
        return Err(type_error2(values, keys, "have wrong length"));
    }
    let Some(first) = items.first() else {
        return Ok(OwnedValue::Null);
    };
    let (mut best, mut best_key) = (first, 0);
    for i in 1..items.len() {
        let less =
            compare_values::<JqSemantics>(&ks[i], &ks[best_key]) == core::cmp::Ordering::Less;
        if less == is_min {
            best = &items[i];
            best_key = i;
        }
    }
    Ok(best.clone())
}

/// A builtin's sub-expressions, in argument order (`map_builtin_subexprs`
/// visits them in the same order).
fn builtin_args(builtin: &Builtin) -> Vec<&Expr> {
    match builtin_kids(builtin) {
        BuiltinKids::None => Vec::new(),
        BuiltinKids::One(a) => vec![a],
        BuiltinKids::Two(a, b) => vec![a, b],
        BuiltinKids::Three(a, b, c) => vec![a, b, c],
    }
}

/// The jq-defined builtins the prelude implements, as `(definition, args)`.
fn prelude_call(builtin: &Builtin) -> Option<(&'static str, Vec<&Expr>)> {
    use Builtin as B;
    let call: (&'static str, Vec<&Expr>) = match builtin {
        B::Select(f) => ("_sp_select", vec![f]),
        B::Map(f) => ("_sp_map", vec![f]),
        B::MapValues(f) => ("_sp_map_values", vec![f]),
        B::Recurse => ("_sp_recurse", vec![]),
        B::RecurseF(f) => ("_sp_recurse", vec![f]),
        B::RecurseCond(f, c) => ("_sp_recurse", vec![f, c]),
        B::Add => ("_sp_add", vec![]),
        B::Del(f) => ("_sp_del", vec![f]),
        B::ToEntries => ("_sp_to_entries", vec![]),
        B::FromEntries => ("_sp_from_entries", vec![]),
        B::WithEntries(f) => ("_sp_with_entries", vec![f]),
        B::Reverse => ("_sp_reverse", vec![]),
        B::Paths => ("_sp_paths", vec![]),
        B::PathsFilter(f) => ("_sp_paths", vec![f]),
        B::Arrays => ("_sp_arrays", vec![]),
        B::Objects => ("_sp_objects", vec![]),
        B::Iterables => ("_sp_iterables", vec![]),
        B::Booleans => ("_sp_booleans", vec![]),
        B::Numbers => ("_sp_numbers", vec![]),
        B::Normals => ("_sp_normals", vec![]),
        B::Finites => ("_sp_finites", vec![]),
        B::Strings => ("_sp_strings", vec![]),
        B::Nulls => ("_sp_nulls", vec![]),
        B::Values => ("_sp_values", vec![]),
        B::Scalars => ("_sp_scalars", vec![]),
        B::Join(x) => ("_sp_join", vec![x]),
        B::Flatten => ("_sp_flatten", vec![]),
        B::FlattenDepth(x) => ("_sp_flatten", vec![x]),
        B::Limit(n, g) => ("_sp_limit", vec![n, g]),
        B::FirstStream(g) => ("_sp_first", vec![g]),
        B::LastStream(g) => ("_sp_last", vec![g]),
        B::NthStream(n, g) => ("_sp_nth", vec![n, g]),
        B::IsEmpty(g) => ("_sp_isempty", vec![g]),
        B::Any => ("_sp_any", vec![]),
        B::All => ("_sp_all", vec![]),
        B::AnyF(c) => ("_sp_any", vec![c]),
        B::AllF(c) => ("_sp_all", vec![c]),
        B::AnyCond(g, c) => ("_sp_any", vec![g, c]),
        B::AllCond(g, c) => ("_sp_all", vec![g, c]),
        B::First => ("_sp_first", vec![]),
        B::Last => ("_sp_last", vec![]),
        B::Nth(n) => ("_sp_nth", vec![n]),
        B::In(xs) => ("_sp_in", vec![xs]),
        B::Inside(xs) => ("_sp_inside", vec![xs]),
        B::UpperIn(s) => ("_sp_IN", vec![s]),
        B::UpperInSrc(src, s) => ("_sp_IN", vec![src, s]),
        B::AsciiDowncase => ("_sp_ascii_downcase", vec![]),
        B::AsciiUpcase => ("_sp_ascii_upcase", vec![]),
        B::Walk(f) => ("_sp_walk", vec![f]),
        B::Pick(f) => ("_sp_pick", vec![f]),
        B::Abs => ("_sp_abs", vec![]),
        B::SortBy(f) => ("_sp_sort_by", vec![f]),
        B::GroupBy(f) => ("_sp_group_by", vec![f]),
        B::Unique => ("_sp_unique", vec![]),
        B::UniqueBy(f) => ("_sp_unique_by", vec![f]),
        B::MinBy(f) => ("_sp_min_by", vec![f]),
        B::MaxBy(f) => ("_sp_max_by", vec![f]),
        B::Indices(i) => ("_sp_indices", vec![i]),
        B::Index(i) => ("_sp_index", vec![i]),
        B::Rindex(i) => ("_sp_rindex", vec![i]),
        B::DebugMsg(m) => ("_sp_debug", vec![m]),
        B::IsFinite => ("_sp_isfinite", vec![]),
        B::HaltError => ("_sp_halt_error", vec![]),
        B::Fromdateiso8601 => ("_sp_fromdateiso8601", vec![]),
        B::Todateiso8601 => ("_sp_todateiso8601", vec![]),
        B::Fromdate => ("_sp_fromdate", vec![]),
        B::Todate => ("_sp_todate", vec![]),
        B::Match(v) => ("_sp_match", vec![v]),
        B::MatchFlags(re, mode) => ("_sp_match", vec![re, mode]),
        B::Test(v) => ("_sp_test", vec![v]),
        B::TestFlags(re, mode) => ("_sp_test", vec![re, mode]),
        B::Capture(v) => ("_sp_capture", vec![v]),
        B::CaptureFlags(re, mods) => ("_sp_capture", vec![re, mods]),
        B::Scan(re) => ("_sp_scan", vec![re]),
        B::ScanFlags(re, flags) => ("_sp_scan", vec![re, flags]),
        B::Splits(re) => ("_sp_splits", vec![re]),
        B::SplitsFlags(re, flags) => ("_sp_splits", vec![re, flags]),
        B::SplitRegex(re, flags) => ("_sp_split", vec![re, flags]),
        B::Sub(re, s) => ("_sp_sub", vec![re, s]),
        B::SubFlags(re, s, flags) => ("_sp_sub", vec![re, s, flags]),
        B::Gsub(re, s) => ("_sp_gsub", vec![re, s]),
        B::GsubFlags(re, s, flags) => ("_sp_gsub", vec![re, s, flags]),
        B::Combinations => ("_sp_combinations", vec![]),
        B::CombinationsN(n) => ("_sp_combinations", vec![n]),
        B::Transpose => ("_sp_transpose", vec![]),
        B::TruncateStream(stream) => ("_sp_truncate_stream", vec![stream]),
        B::FromStream(i) => ("_sp_fromstream", vec![i]),
        B::ToStream => ("_sp_tostream", vec![]),
        B::BSearch(target) => ("_sp_bsearch", vec![target]),
        B::UpperIndex(idx) => ("_sp_INDEX", vec![idx]),
        B::UpperIndexStream(stream, idx) => ("_sp_INDEX", vec![stream, idx]),
        B::LeafPaths => ("_sp_leaf_paths", vec![]),
        _ => return None,
    };
    Some(call)
}

/// Builtins the interpreter refuses outright: succinctly extensions with no
/// jq 1.7.1 model, and yq-only ones that reach path mode.
///
/// The input builtins and `halt_error` are C builtins like any other: the
/// shipped ones read the CLI's input queue, and `halt_error` writes its own
/// message before it halts.
fn unsupported_builtin(builtin: &Builtin) -> Option<&'static str> {
    use Builtin as B;
    Some(match builtin {
        B::AtOffset(_) | B::AtPosition(..) => "at_offset/at_position (a succinctly extension)",
        B::PathNoArg | B::Parent | B::ParentN(_) => "a yq path builtin",
        B::IsValid(_) | B::Skip(..) | B::AnyC(..) | B::AllC(..) | B::RecurseDown => {
            "a builtin jq 1.7.1 does not define"
        }
        _ => return None,
    })
}

/// `parse_slice` for an array of `len` elements (or a string of `len`
/// codepoints): the `[start, end)` range a
/// `{"start": s, "end": e}` key selects (`null` bounds default; the start
/// rounds down and the end up). `None` when a bound is not a number, which
/// `jv_get` reports as an error.
fn slice_bounds(len: usize, bounds: &IndexMap<String, OwnedValue>) -> Option<(usize, usize)> {
    let bound = |name: &str, default: f64| -> Option<f64> {
        match bounds.get(name) {
            None | Some(OwnedValue::Null) => Some(default),
            Some(v) if v.is_number() => number_f64(v),
            Some(_) => None,
        }
    };
    let n = len as f64;
    let (mut dstart, mut dend) = (bound("start", 0.0)?, bound("end", n)?);
    if dstart.is_nan() {
        dstart = 0.0;
    }
    if dstart < 0.0 {
        dstart += n;
    }
    let start = dstart.clamp(0.0, n) as usize;
    if dend.is_nan() {
        dend = n;
    }
    if dend < 0.0 {
        dend += n;
    }
    let mut end = if dend < 0.0 {
        start
    } else {
        dend.min(f64::from(i32::MAX)) as usize
    };
    end = end.min(len);
    if end < len && (end as f64) < dend {
        end += 1;
    }
    Some((start, end.max(start)))
}

/// A number's double, `None` for anything else.
fn number_f64(value: &OwnedValue) -> Option<f64> {
    if value.is_number() {
        value.as_f64()
    } else {
        None
    }
}

/// `-1` is `_negate` applied to the literal `1` at run time (`'-' Exp`), so a
/// negative number is computed, never a constant.
fn literal_value(lit: &Literal) -> Option<OwnedValue> {
    Some(match lit {
        Literal::Null => OwnedValue::Null,
        Literal::Bool(b) => OwnedValue::Bool(*b),
        Literal::NumberLiteral(_, spelling) if spelling.starts_with('-') => return None,
        Literal::NumberLiteral(repr, spelling) => literal(*repr, spelling),
        // No spelling: a negation the parser folded (`-01`), which jq
        // computes with `_negate`.
        Literal::Int(_) | Literal::Float(_) => return None,
        Literal::String(s) => string(s),
    })
}

/// What jq compiles to one constant (`LOADK`): a literal, and arrays and
/// objects of constants that `gen_const_array`/`gen_const_object` fold, and a
/// binary operator over two constants (`constant_fold`).
fn constant_value(expr: &Expr) -> Option<OwnedValue> {
    match expr {
        Expr::Literal(lit) => literal_value(lit),
        Expr::Paren(inner) => constant_value(inner),
        Expr::Loc { line, file } => {
            let mut map = IndexMap::new();
            map.insert(
                String::from("file"),
                string(file.as_deref().unwrap_or("<top-level>")),
            );
            map.insert(
                String::from("line"),
                literal(NumberRepr::Int(*line as i64), &line.to_string()),
            );
            Some(OwnedValue::Object(map.into()))
        }
        Expr::Array(inner) => {
            let elements: Vec<&Expr> = match &**inner {
                Expr::Comma(items) => items.iter().collect(),
                single => vec![single],
            };
            let values: Option<Vec<OwnedValue>> =
                elements.into_iter().map(constant_value).collect();
            Some(OwnedValue::Array(values?.into()))
        }
        Expr::Object(entries) => {
            let mut built: IndexMap<String, OwnedValue> = IndexMap::new();
            for entry in entries {
                let ObjectKey::Literal(key) = &entry.key else {
                    return None;
                };
                built.insert(key.clone(), constant_value(&entry.value)?);
            }
            Some(OwnedValue::Object(built.into()))
        }
        Expr::Arithmetic {
            op, left, right, ..
        } => {
            let (l, r) = (constant_value(left)?, constant_value(right)?);
            if let Some(kept) = binop_keeps(*op, &l, &r) {
                return Some(kept.clone());
            }
            arith_combine::<JqSemantics>(*op, l.clone(), r.clone())
                .ok()
                .map(|v| fresh(v, &[&l, &r]))
        }
        Expr::Compare { op, left, right } => {
            let (l, r) = (constant_value(left)?, constant_value(right)?);
            Some(OwnedValue::Bool(apply_compare_op::<JqSemantics>(
                *op, &l, &r,
            )))
        }
        _ => None,
    }
}

fn pattern_vars<'e>(pattern: &'e Pattern, out: &mut Vec<&'e str>) {
    match pattern {
        Pattern::Var(name) => {
            if !out.contains(&name.as_str()) {
                out.push(name);
            }
        }
        Pattern::Array(items) => items.iter().for_each(|p| pattern_vars(p, out)),
        Pattern::Object(entries) => {
            for entry in entries {
                if let Some(name) = &entry.bind {
                    if !out.contains(&name.as_str()) {
                        out.push(name);
                    }
                }
                pattern_vars(&entry.pattern, out);
            }
        }
    }
}

/// `.[n]`'s key: a program literal, `_negate` of one for a negative index,
/// or the number it was spelled as (`.[1.0]` records `1.0`).
fn index_key(idx: i64, spelled: Option<&NumberKey>) -> OwnedValue {
    spelled.map_or_else(|| literal_int(idx), number_key)
}

/// A spelled number key: a source literal keeps its spelling and is
/// allocated; one the parser computed (a negated literal) is a double.
fn number_key(key: &NumberKey) -> OwnedValue {
    match key {
        NumberKey::Float(f) => OwnedValue::Float(*f),
        NumberKey::Literal(f, spelling) => literal(NumberRepr::Float(*f), spelling),
    }
}

/// A static slice's key as `gen_slice_index` builds it: each bound's own
/// spelling when it kept one, the raw value jq never parsed (a `null`
/// target), else the integer literal (`null` when absent); or, for a
/// descriptor the program named whole (`SliceBoundKey::Verbatim`), that
/// object as written.
fn static_slice_key(
    start: Option<i64>,
    end: Option<i64>,
    start_key: Option<&SliceBoundKey>,
    end_key: Option<&SliceBoundKey>,
) -> OwnedValue {
    if let Some(SliceBoundKey::Verbatim(descriptor)) = start_key {
        return fresh((**descriptor).clone(), &[]);
    }
    let bound = |n: Option<i64>, key: Option<&SliceBoundKey>| match key {
        Some(SliceBoundKey::Number(k)) => Some(number_key(k)),
        Some(SliceBoundKey::Raw(value) | SliceBoundKey::Verbatim(value)) => {
            Some(fresh((**value).clone(), &[]))
        }
        None => n.map(literal_int),
    };
    slice_key(bound(start, start_key), bound(end, end_key))
}

fn literal_int(n: i64) -> OwnedValue {
    if n < 0 {
        OwnedValue::Int(n)
    } else {
        literal(NumberRepr::Int(n), &n.to_string())
    }
}

/// `gen_slice_index`'s key, `{"start": s, "end": e}` (an absent bound is
/// `null`), fresh per evaluation.
fn slice_key(start: Option<OwnedValue>, end: Option<OwnedValue>) -> OwnedValue {
    let mut map = IndexMap::new();
    map.insert(String::from("start"), start.unwrap_or(OwnedValue::Null));
    map.insert(String::from("end"), end.unwrap_or(OwnedValue::Null));
    OwnedValue::Object(map.into())
}

fn number_owned(value: f64) -> OwnedValue {
    if value.fract() == 0.0 && value.abs() < 9.0e15 {
        OwnedValue::Int(value as i64)
    } else {
        OwnedValue::Float(value)
    }
}

/// Emit what the existing evaluator answers for C builtin `builtin` (called
/// as `expr`, its arguments' values `args` in place) on `input`: the input or
/// argument itself when the builtin hands it back unchanged ([`kept`]), a
/// fresh value otherwise. The call runs either way, for its side effects
/// (`debug`) and its errors.
fn emit_c_builtin(
    expr: &Expr,
    builtin: &Builtin,
    input: &OwnedValue,
    args: &[OwnedValue],
    reg: &Reg,
    sink: &mut Sink<'_>,
) -> R {
    let keep = kept(builtin, input, args);
    // A kept answer is known without the call, and only `debug`/`stderr`
    // have a side effect to run it for.
    if let Some(k) = keep {
        if !matches!(builtin, Builtin::Debug | Builtin::Stderr) {
            let value = match k {
                Kept::Input => input.clone(),
                Kept::Arg(i) => args[i].clone(),
            };
            return sink(value, reg.clone());
        }
    }
    let (outputs, control) = delegate_eval(expr, input);
    let mut inputs: Vec<&OwnedValue> = Vec::with_capacity(args.len() + 1);
    inputs.push(input);
    inputs.extend(args);
    for v in outputs {
        let value = match keep {
            Some(Kept::Input) => input.clone(),
            Some(Kept::Arg(i)) => args[i].clone(),
            None => fresh(v, &inputs),
        };
        sink(value, reg.clone())?;
    }
    match control {
        Some(control) => Err(control_raise(control)),
        None => Ok(()),
    }
}

/// Emit what the existing evaluator answers for `expr` on `input`, each
/// output fresh (against `input` and the argument values `args`) and at
/// `reg`.
fn emit_delegated(
    expr: &Expr,
    input: &OwnedValue,
    args: &[&OwnedValue],
    reg: &Reg,
    sink: &mut Sink<'_>,
) -> R {
    let (outputs, control) = delegate_eval(expr, input);
    let mut inputs = Vec::with_capacity(args.len() + 1);
    inputs.push(input);
    inputs.extend_from_slice(args);
    for v in outputs {
        sink(fresh(v, &inputs), reg.clone())?;
    }
    match control {
        Some(control) => Err(control_raise(control)),
        None => Ok(()),
    }
}

/// Run `expr` on `input` through the existing evaluator: its outputs, and
/// the escape that ended it.
fn delegate_eval(expr: &Expr, input: &OwnedValue) -> (Vec<OwnedValue>, Option<Control>) {
    let result = eval_owned_with_file_index::<Vec<u64>, JqSemantics>(expr, input, &[]);
    match result {
        QueryResult::One(v) => match super::eval_generic::to_owned::<JqSemantics, _>(&v) {
            Ok(o) => (vec![o], None),
            Err(e) => (vec![], Some(Control::Error(e))),
        },
        QueryResult::OneCursor(c) => {
            match super::eval_generic::to_owned::<JqSemantics, _>(&c.value()) {
                Ok(o) => (vec![o], None),
                Err(e) => (vec![], Some(Control::Error(e))),
            }
        }
        QueryResult::Many(vs) => {
            let mut out = Vec::new();
            for v in &vs {
                match super::eval_generic::to_owned::<JqSemantics, _>(v) {
                    Ok(o) => out.push(o),
                    Err(e) => return (out, Some(Control::Error(e))),
                }
            }
            (out, None)
        }
        QueryResult::None => (vec![], None),
        QueryResult::Error(e) => (vec![], Some(Control::Error(e))),
        QueryResult::Owned(v) => (vec![v], None),
        QueryResult::ManyOwned(vs) => (vs, None),
        QueryResult::Break(label) => (vec![], Some(Control::Break(label))),
        QueryResult::Halt(code) => (vec![], Some(Control::Halt(code))),
        QueryResult::Partial(vs, control) => (vs, Some(control)),
    }
}

fn control_raise(control: Control) -> Raise {
    match control {
        Control::Error(e) => raise_eval(e),
        Control::Break(label) => unsupported(format!("a delegated break to {label}")), // patchcov: coverage tolerate-line reason="unreachable: delegate() declines any expression holding a break before delegating it"
        Control::Halt(code) => Raise::Halt(code),
    }
}

fn variant_name(expr: &Expr) -> String {
    let debug = match expr {
        Expr::Builtin(builtin) => format!("{builtin:?}"),
        other => format!("{other:?}"), // patchcov: coverage tolerate-line reason="defensive: only reached for an Expr variant the interpreter does not implement"
    };
    debug
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("?")
        .to_string()
}

// The interpreter reads storage identity, which the `unshared-containers`
// holdout does not have (every clone copies), so its tests pin the shipped
// shape only.
#[cfg(all(test, not(feature = "unshared-containers")))]
mod reference;

#[cfg(all(test, not(feature = "unshared-containers")))]
mod tests;
