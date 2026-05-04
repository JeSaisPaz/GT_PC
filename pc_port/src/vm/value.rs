use std::fmt;
use std::rc::Rc;

/// Adhoc value type — tagged union matching the bytecode type system.
#[derive(Clone)]
pub enum Value {
    Nil,
    Void,
    Bool(bool),
    Int(i32),
    UInt(u32),
    Long(i64),
    ULong(u64),
    Float(f32),
    Double(f64),
    Byte(i8),
    UByte(u8),
    Short(i16),
    UShort(u16),
    String(Rc<String>),
    Symbol(Rc<String>),
    Array(Rc<Vec<Value>>),
    Map(Rc<Vec<(Value, Value)>>),
    Object(Rc<ObjectInstance>),
    Function(Rc<FunctionValue>),
    Native(Rc<NativeFn>),
}

#[derive(Clone)]
pub struct ObjectInstance {
    pub class_path: String,
    pub fields: Vec<Value>,
}

impl ObjectInstance {
    pub fn simple(class_path: &str) -> Self {
        ObjectInstance { class_path: class_path.to_string(), fields: vec![] }
    }
}

#[derive(Clone)]
pub struct FunctionValue {
    pub module_path: String,
    pub name: String,
    pub code_frame: usize,
    pub is_method: bool,
    pub static_base: usize,
    pub parent_frame: usize,
}

pub type NativeFn = Rc<dyn Fn(&[Value]) -> Value>;

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Void => write!(f, "void"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::UInt(u) => write!(f, "{}", u),
            Value::Long(l) => write!(f, "{}l", l),
            Value::ULong(u) => write!(f, "{}ul", u),
            Value::Float(fl) => write!(f, "{}f", fl),
            Value::Double(d) => write!(f, "{}", d),
            Value::Byte(b) => write!(f, "{}b", b),
            Value::UByte(b) => write!(f, "{}ub", b),
            Value::Short(s) => write!(f, "{}s", s),
            Value::UShort(u) => write!(f, "{}us", u),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Symbol(s) => write!(f, "'{}", s),
            Value::Array(arr) => write!(f, "[{:?}]", arr),
            Value::Map(_m) => write!(f, "{{...}}"),
            Value::Object(o) => write!(f, "{}#{{...}}", o.class_path),
            Value::Function(fv) => write!(f, "fn {}::{}", fv.module_path, fv.name),
            Value::Native(_) => write!(f, "<native fn>"),
        }
    }
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Nil | Value::Void => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::UInt(u) => *u != 0,
            Value::Long(l) => *l != 0,
            Value::ULong(u) => *u != 0,
            Value::Float(f) => *f != 0.0,
            Value::Double(d) => *d != 0.0,
            Value::Byte(b) => *b != 0,
            Value::UByte(b) => *b != 0,
            Value::Short(s) => *s != 0,
            Value::UShort(u) => *u != 0,
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Value::Int(i) => Some(*i),
            Value::UInt(u) => Some(*u as i32),
            Value::Byte(b) => Some(*b as i32),
            Value::UByte(b) => Some(*b as i32),
            Value::Short(s) => Some(*s as i32),
            Value::UShort(u) => Some(*u as i32),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Double(d) => Some(*d as f32),
            Value::Int(i) => Some(*i as f32),
            Value::UInt(u) => Some(*u as f32),
            Value::Byte(b) => Some(*b as f32),
            Value::UByte(b) => Some(*b as f32),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Value::String(s) => s.to_string(),
            Value::Int(i) => i.to_string(),
            Value::UInt(u) => u.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Double(d) => d.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Nil => String::new(),
            Value::Symbol(s) => s.to_string(),
            _ => format!("{:?}", self),
        }
    }
}
