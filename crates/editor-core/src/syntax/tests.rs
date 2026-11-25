//! Text objects and motions on real code in each language. Snippets carry non-ASCII chars,
//! so byte offsets that leak through as char offsets fail.

use std::path::Path;

use super::*;
use crate::buffer::{Buffer, Pos};

/// The char position of the first `needle` in `src`.
fn at(src: &str, needle: &str) -> Pos {
    let byte = src.find(needle).unwrap_or_else(|| panic!("{needle:?} not in the snippet"));
    let before = &src[..byte];
    let line = before.matches('\n').count();
    let col = before.rsplit('\n').next().unwrap_or("").chars().count();
    Pos::new(line, col)
}

/// What a range covers: `L<first>-<last>` for whole lines, else the text.
fn show(b: &Buffer, r: (usize, usize, bool)) -> String {
    if r.2 { format!("L{}-{}", b.pos(r.0).line, b.pos(r.1).line) } else { b.slice(r.0, r.1) }
}

fn obj(src: &str, file: &str, needle: &str, o: Obj, inner: bool, count: usize) -> Option<String> {
    let b = Buffer::from_str(src);
    object(&b, Some(Path::new(file)), at(src, needle), o, inner, count).map(|r| show(&b, r))
}

fn jmp(src: &str, file: &str, from: Pos, o: Obj, forward: bool, end: bool, count: usize) -> Option<Pos> {
    let b = Buffer::from_str(src);
    jump(&b, Some(Path::new(file)), from, o, forward, end, count)
}

const RUST: &str = "use std::fmt;

/// Greets someone. Ünïcode in the doc.
#[inline]
pub fn greet(name: &str, times: u32) -> String {
    let shout = |s: &str| s.to_uppercase();
    pad(shout(name), times as usize, \"héllo\")
}

struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn len(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
";

#[test]
fn rust_functions_take_doc_comments_and_attributes() {
    let f = "src/lib.rs";
    assert_eq!(obj(RUST, f, "pad(", Obj::Function, false, 1).as_deref(), Some("L2-7"));
    assert_eq!(obj(RUST, f, "Greets", Obj::Function, false, 1).as_deref(), Some("L2-7"), "the doc comment belongs to the function");
    assert_eq!(obj(RUST, f, "pad(", Obj::Function, true, 1).as_deref(), Some("L5-6"));
    assert_eq!(obj(RUST, f, "to_uppercase", Obj::Function, false, 1).as_deref(), Some("|s: &str| s.to_uppercase()"));
    assert_eq!(obj(RUST, f, "to_uppercase", Obj::Function, true, 1).as_deref(), Some("s.to_uppercase()"));
    assert_eq!(obj(RUST, f, "to_uppercase", Obj::Function, false, 2).as_deref(), Some("L2-7"), "a count reaches the enclosing function");
    assert_eq!(obj(RUST, f, "to_uppercase", Obj::Function, false, 3), None);
    assert_eq!(obj(RUST, f, "sqrt", Obj::Function, true, 1).as_deref(), Some("L16-16"));
}

#[test]
fn rust_classes_are_structs_and_impls() {
    let f = "src/lib.rs";
    assert_eq!(obj(RUST, f, "y: f64", Obj::Class, false, 1).as_deref(), Some("L9-12"));
    assert_eq!(obj(RUST, f, "y: f64", Obj::Class, true, 1).as_deref(), Some("L10-11"));
    assert_eq!(obj(RUST, f, "sqrt", Obj::Class, false, 1).as_deref(), Some("L14-18"));
    assert_eq!(obj(RUST, f, "sqrt", Obj::Class, true, 1).as_deref(), Some("L15-17"));
    assert_eq!(obj(RUST, f, "pad(", Obj::Class, false, 1), None);
}

#[test]
fn rust_parameters_take_their_comma() {
    let f = "src/lib.rs";
    assert_eq!(obj(RUST, f, "name: &str", Obj::Parameter, true, 1).as_deref(), Some("name: &str"));
    assert_eq!(obj(RUST, f, "name: &str", Obj::Parameter, false, 1).as_deref(), Some("name: &str, "));
    assert_eq!(
        obj(RUST, f, "times: u32", Obj::Parameter, false, 1).as_deref(),
        Some(", times: u32"),
        "the last one takes the comma before it"
    );
    assert_eq!(obj(RUST, f, "name), ", Obj::Parameter, true, 1).as_deref(), Some("name"));
    assert_eq!(obj(RUST, f, "name), ", Obj::Parameter, true, 2).as_deref(), Some("shout(name)"), "a count reaches the enclosing argument");
    assert_eq!(obj(RUST, f, "name), ", Obj::Parameter, false, 2).as_deref(), Some("shout(name), "));
    assert_eq!(obj(RUST, f, "\"héllo\"", Obj::Parameter, false, 1).as_deref(), Some(", \"héllo\""));
    assert_eq!(obj(RUST, f, ", times", Obj::Parameter, true, 1).as_deref(), Some("times: u32"), "on a comma: the next one");
}

#[test]
fn rust_comments_without_markers() {
    let f = "src/lib.rs";
    assert_eq!(obj(RUST, f, "Greets", Obj::Comment, false, 1).as_deref(), Some("L2-2"));
    assert_eq!(obj(RUST, f, "Greets", Obj::Comment, true, 1).as_deref(), Some("Greets someone. Ünïcode in the doc."));
    assert_eq!(obj(RUST, f, "fmt", Obj::Comment, false, 1), None);
}

#[test]
fn rust_jumps_count_and_stop_at_the_last() {
    let f = "src/lib.rs";
    let top = Pos::new(0, 0);
    assert_eq!(jmp(RUST, f, top, Obj::Function, true, false, 1), Some(Pos::new(4, 0)));
    assert_eq!(jmp(RUST, f, top, Obj::Function, true, false, 2), Some(Pos::new(5, 16)));
    assert_eq!(jmp(RUST, f, top, Obj::Function, true, false, 3), Some(Pos::new(15, 4)));
    assert_eq!(jmp(RUST, f, top, Obj::Function, true, false, 9), Some(Pos::new(15, 4)), "past the last, the last");
    assert_eq!(jmp(RUST, f, Pos::new(19, 0), Obj::Function, false, false, 1), Some(Pos::new(15, 4)));
    assert_eq!(jmp(RUST, f, top, Obj::Function, true, true, 1).map(|p| p.line), Some(5), "the closure ends first");
    assert_eq!(jmp(RUST, f, top, Obj::Function, true, true, 2), Some(Pos::new(7, 0)));
    assert_eq!(jmp(RUST, f, top, Obj::Class, true, false, 1), Some(Pos::new(9, 0)));
    assert_eq!(jmp(RUST, f, Pos::new(9, 0), Obj::Class, true, false, 1), Some(Pos::new(14, 0)));
    assert_eq!(jmp(RUST, f, top, Obj::Comment, true, false, 1), Some(Pos::new(2, 0)));
}

const PY: &str = "import os


@cache
def load(path, mode=\"r\"):
    \"\"\"Read a fïle.\"\"\"
    with open(path, mode) as f:
        return f.read()


class Store:
    class Meta:
        name = \"store\"

    def get(self, key):
        # Lookup.
        # Fast.
        return self.data[key]
";

#[test]
fn python_decorators_and_nested_classes() {
    let f = "app.py";
    assert_eq!(obj(PY, f, "f.read", Obj::Function, false, 1).as_deref(), Some("L3-7"));
    assert_eq!(obj(PY, f, "@cache", Obj::Function, false, 1).as_deref(), Some("L3-7"));
    assert_eq!(obj(PY, f, "f.read", Obj::Function, true, 1).as_deref(), Some("L5-7"));
    assert_eq!(obj(PY, f, "name = ", Obj::Class, false, 1).as_deref(), Some("L11-12"));
    assert_eq!(obj(PY, f, "name = ", Obj::Class, false, 2).as_deref(), Some("L10-17"));
    assert_eq!(obj(PY, f, "name = ", Obj::Class, true, 2).as_deref(), Some("L11-17"));
    assert_eq!(obj(PY, f, "self.data", Obj::Function, false, 1).as_deref(), Some("L14-17"));
    assert_eq!(obj(PY, f, "mode=", Obj::Parameter, true, 1).as_deref(), Some("mode=\"r\""));
    assert_eq!(obj(PY, f, "mode=", Obj::Parameter, false, 1).as_deref(), Some(", mode=\"r\""));
    assert_eq!(obj(PY, f, "Fast", Obj::Comment, false, 1).as_deref(), Some("L15-16"), "a run of line comments");
    assert_eq!(obj(PY, f, "Fast", Obj::Comment, true, 1).as_deref(), Some("Fast."));
}

const JS: &str = "// Helpers for the äpp.
// Second line.

/** Adds two numbers. */
export function add(a, b) {
  return a + b;
}

const twice = (x) => x * 2;

class Counter {
  increment(by = 1) {
    this.n += by;
  }
}
";

#[test]
fn javascript_functions_arrows_methods_and_classes() {
    let f = "util.js";
    assert_eq!(obj(JS, f, "a + b", Obj::Function, false, 1).as_deref(), Some("L3-6"), "with its JSDoc and export");
    assert_eq!(obj(JS, f, "a + b", Obj::Function, true, 1).as_deref(), Some("L5-5"));
    assert_eq!(obj(JS, f, "x * 2", Obj::Function, false, 1).as_deref(), Some("(x) => x * 2"));
    assert_eq!(obj(JS, f, "x * 2", Obj::Function, true, 1).as_deref(), Some("x * 2"));
    assert_eq!(obj(JS, f, "this.n", Obj::Function, false, 1).as_deref(), Some("L11-13"));
    assert_eq!(obj(JS, f, "this.n", Obj::Class, false, 1).as_deref(), Some("L10-14"));
    assert_eq!(obj(JS, f, "this.n", Obj::Class, true, 1).as_deref(), Some("L11-13"));
    assert_eq!(obj(JS, f, "by = 1", Obj::Parameter, true, 1).as_deref(), Some("by = 1"));
    assert_eq!(obj(JS, f, "Second", Obj::Comment, false, 1).as_deref(), Some("L0-1"));
    assert_eq!(obj(JS, f, "Second", Obj::Comment, true, 1).as_deref(), Some("Second line."));
    assert_eq!(obj(JS, f, "Adds", Obj::Comment, true, 1).as_deref(), Some("Adds two numbers."));
}

const TS: &str = "interface Shape {
  area(): number;
  name: string;
}

type Pt = { x: number; y: number };

enum Cölor { Red, Green }

function make(s: Shape, label?: string): void {}
";

#[test]
fn typescript_interfaces_aliases_and_enums() {
    let f = "shape.ts";
    assert_eq!(obj(TS, f, "name: string", Obj::Class, false, 1).as_deref(), Some("L0-3"));
    assert_eq!(obj(TS, f, "name: string", Obj::Class, true, 1).as_deref(), Some("L1-2"));
    assert_eq!(obj(TS, f, "x: number", Obj::Class, false, 1).as_deref(), Some("L5-5"));
    assert_eq!(obj(TS, f, "x: number", Obj::Class, true, 1).as_deref(), Some("x: number; y: number"));
    assert_eq!(obj(TS, f, "Green", Obj::Class, true, 1).as_deref(), Some("Red, Green"));
    assert_eq!(obj(TS, f, "label", Obj::Parameter, false, 1).as_deref(), Some(", label?: string"));
    assert_eq!(obj(TS, f, "void", Obj::Function, true, 1).as_deref(), Some(""));
}

#[test]
fn tsx_components_parse_with_jsx() {
    let src = "export function App({ name }: Props) {\n  return <div>{name}</div>;\n}\n";
    assert_eq!(obj(src, "App.tsx", "div", Obj::Function, false, 1).as_deref(), Some("L0-2"));
    assert_eq!(obj(src, "App.tsx", "div", Obj::Function, true, 1).as_deref(), Some("L1-1"));
}

const GO: &str = "package main

// Point is a place.
type Point struct {
	X, Y int
}

// Len returns the length.
func (p Point) Len() float64 {
	return math.Sqrt(float64(p.X*p.X + p.Y*p.Y))
}

func main() {
	f := func(a int) int { return a }
	fmt.Println(f(1), \"é\")
}
";

#[test]
fn go_types_methods_and_literals() {
    let f = "main.go";
    assert_eq!(obj(GO, f, "X, Y", Obj::Class, false, 1).as_deref(), Some("L2-5"));
    assert_eq!(obj(GO, f, "X, Y", Obj::Class, true, 1).as_deref(), Some("L4-4"));
    assert_eq!(obj(GO, f, "math.Sqrt", Obj::Function, false, 1).as_deref(), Some("L7-10"));
    assert_eq!(obj(GO, f, "math.Sqrt", Obj::Function, true, 1).as_deref(), Some("L9-9"));
    assert_eq!(obj(GO, f, "return a", Obj::Function, false, 1).as_deref(), Some("func(a int) int { return a }"));
    assert_eq!(obj(GO, f, "return a", Obj::Function, true, 1).as_deref(), Some("return a"));
    assert_eq!(obj(GO, f, "\"é\"", Obj::Parameter, false, 1).as_deref(), Some(", \"é\""));
    assert_eq!(obj(GO, f, "f(1)", Obj::Parameter, false, 1).as_deref(), Some("f(1), "));
}

const C: &str = "#include <stdio.h>

/** A pöint. */
struct point {
    int x;
    int y;
};

static int sum(int a, int b) {
    return a + b;
}

struct point origin;
";

#[test]
fn c_structs_and_functions() {
    let f = "geo.c";
    assert_eq!(obj(C, f, "int x", Obj::Class, false, 1).as_deref(), Some("L2-6"));
    assert_eq!(obj(C, f, "int x", Obj::Class, true, 1).as_deref(), Some("L4-5"));
    assert_eq!(obj(C, f, "origin", Obj::Class, false, 1), None, "a use of the type is no class");
    assert_eq!(obj(C, f, "a + b", Obj::Function, false, 1).as_deref(), Some("L8-10"));
    assert_eq!(obj(C, f, "int b", Obj::Parameter, false, 1).as_deref(), Some(", int b"));
}

#[test]
fn cpp_classes_and_lambdas() {
    let src =
        "namespace geo {\nclass Shape {\npublic:\n    virtual double area() const = 0;\n};\n}\n\nauto sq = [](int v) { return v * v; };\n";
    let f = "geo.cpp";
    assert_eq!(obj(src, f, "virtual", Obj::Class, false, 1).as_deref(), Some("L1-4"));
    assert_eq!(obj(src, f, "v * v", Obj::Function, false, 1).as_deref(), Some("[](int v) { return v * v; }"));
    assert_eq!(obj(src, f, "v * v", Obj::Function, true, 1).as_deref(), Some("return v * v;"));
}

const JAVA: &str = "/** A greeter. */
@Deprecated
public class Greeter {
    private final String name;

    /** Builds öne. */
    public Greeter(String name) {
        this.name = name;
    }

    String greet(String other, int times) {
        return \"hi \" + other;
    }
}
";

#[test]
fn java_classes_constructors_and_methods() {
    let f = "Greeter.java";
    assert_eq!(obj(JAVA, f, "private final", Obj::Class, false, 1).as_deref(), Some("L0-13"));
    assert_eq!(obj(JAVA, f, "private final", Obj::Class, true, 1).as_deref(), Some("L3-12"));
    assert_eq!(obj(JAVA, f, "this.name", Obj::Function, false, 1).as_deref(), Some("L5-8"));
    assert_eq!(obj(JAVA, f, "this.name", Obj::Function, true, 1).as_deref(), Some("L7-7"));
    assert_eq!(obj(JAVA, f, "int times", Obj::Parameter, false, 1).as_deref(), Some(", int times"));
    assert_eq!(obj(JAVA, f, "\"hi \"", Obj::Function, false, 1).as_deref(), Some("L10-12"));
}

const RUBY: &str = "# A widget.
class Widget
  # Draws ït.
  def draw(canvas, scale = 1)
    canvas.paint(self)
  end
end
";

#[test]
fn ruby_classes_and_methods() {
    let f = "widget.rb";
    assert_eq!(obj(RUBY, f, "canvas.paint", Obj::Class, false, 1).as_deref(), Some("L0-6"));
    assert_eq!(obj(RUBY, f, "canvas.paint", Obj::Function, false, 1).as_deref(), Some("L2-5"));
    assert_eq!(obj(RUBY, f, "canvas.paint", Obj::Function, true, 1).as_deref(), Some("L4-4"));
    assert_eq!(obj(RUBY, f, "scale = 1", Obj::Parameter, false, 1).as_deref(), Some(", scale = 1"));
}

#[test]
fn bash_and_lua_functions() {
    let sh = "# Build ït.\nbuild() {\n  cargo build --release\n}\n";
    assert_eq!(obj(sh, "make.sh", "cargo", Obj::Function, false, 1).as_deref(), Some("L0-3"));
    assert_eq!(obj(sh, "make.sh", "cargo", Obj::Function, true, 1).as_deref(), Some("L2-2"));
    let lua = "--- Adds.\nlocal function add(a, b)\n  return a + b\nend\n";
    assert_eq!(obj(lua, "m.lua", "a + b", Obj::Function, false, 1).as_deref(), Some("L0-3"));
    assert_eq!(obj(lua, "m.lua", "a + b", Obj::Function, true, 1).as_deref(), Some("L2-2"));
    assert_eq!(obj(lua, "m.lua", "b)", Obj::Parameter, false, 1).as_deref(), Some(", b"));
}

#[test]
fn json_has_comments_and_nothing_else() {
    let src = "{\n  // The näme.\n  \"name\": \"x\"\n}\n";
    assert_eq!(obj(src, "tsconfig.json", "näme", Obj::Comment, true, 1).as_deref(), Some("The näme."));
    assert_eq!(obj(src, "tsconfig.json", "name\"", Obj::Function, false, 1), None);
}

#[test]
fn odd_input_gives_none_without_panics() {
    for (src, file) in [("", "a.rs"), ("fn broken( {", "a.rs"), ("def (", "a.py"), ("}}}{{{", "a.go"), ("\n\n\n", "a.c")] {
        let b = Buffer::from_str(src);
        for o in [Obj::Function, Obj::Class, Obj::Parameter, Obj::Comment] {
            for inner in [false, true] {
                let _ = object(&b, Some(Path::new(file)), Pos::new(0, 0), o, inner, 1);
                let _ = object(&b, Some(Path::new(file)), Pos::new(99, 99), o, inner, 3);
            }
            let _ = jump(&b, Some(Path::new(file)), Pos::new(99, 99), o, false, true, 2);
        }
    }
    let b = Buffer::from_str("fn a() {}\n");
    assert_eq!(object(&b, Some(Path::new("notes.txt")), Pos::new(0, 0), Obj::Function, false, 1), None);
    assert_eq!(object(&b, None, Pos::new(0, 0), Obj::Function, false, 1), None);
    assert!(supported(Some(Path::new("x.rs"))) && !supported(Some(Path::new("x.txt"))));
}

#[test]
fn an_edit_reparses_and_the_cache_follows_the_version() {
    let path = Path::new("cache.rs");
    let mut b = Buffer::from_str("fn a() {}\n");
    assert_eq!(object(&b, Some(path), Pos::new(0, 0), Obj::Function, false, 1).map(|r| show(&b, r)).as_deref(), Some("L0-0"));
    b.insert(Pos::new(1, 0), "\nfn b() {\n    1\n}\n");
    let r = object(&b, Some(path), Pos::new(3, 4), Obj::Function, false, 1).map(|r| show(&b, r));
    assert_eq!(r.as_deref(), Some("L2-4"));
}
