use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt,
    hash::Hash,
    rc::Rc,
};

use crate::*;

#[derive(Debug)]
pub struct LoxClass {
    pub name:       LoxStr,
    pub superclass: Option<Rc<LoxClass>>,
    pub methods:    HashMap<LoxStr, Rc<LoxFn>>,
}

impl LoxClass {
    pub fn new<S>(
        name: S,
        superclass: Option<Rc<LoxClass>>,
        methods: HashMap<LoxStr, Rc<LoxFn>>,
    ) -> Self
    where
        S: Into<LoxStr>,
    {
        LoxClass {
            name: name.into(),
            superclass,
            methods,
        }
    }

    pub fn find_method<K>(&self, instance: &LoxInstance, name: &K) -> Option<Rc<LoxFn>>
    where
        K: Eq + Hash,
        LoxStr: Borrow<K>,
    {
        self.methods
            .get(name)
            .map(|method| Rc::new(method.bind(instance.clone())))
            .or_else(|| self.superclass.as_ref().and_then(|sc| sc.find_method(instance, name)))
    }
}

impl fmt::Display for LoxClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<class {}>", self.name)
    }
}

impl Callable for Rc<LoxClass> {
    fn call(&self, interp: &mut Interpreter, args: Vec<Value>) -> Result<Value, LoxError> {
        let instance = LoxInstance::new(self.clone());
        if let Some(init) = self.methods.get("init".as_bytes()) {
            init.bind(instance.clone()).call(interp, args)?;
        }
        Ok(Value::Instance(instance).into())
    }

    fn arity(&self) -> usize {
        self.methods.get("init".as_bytes()).map(|init| init.arity()).unwrap_or(0)
    }
}
