#[derive(Debug)]
pub struct Context<E, W> {
    pub errors: Vec<E>,
    pub warnings: Vec<W>,
}

impl<E, W> Default for Context<E, W> {
    fn default() -> Self {
        Self {
            errors: vec![],
            warnings: vec![],
        }
    }
}

impl<E, W> Context<E, W> {
    pub fn error(&mut self, e: E) {
        self.errors.push(e);
    }

    pub fn warn(&mut self, w: W) {
        self.warnings.push(w);
    }

    pub fn recover<T>(&mut self, r: Result<T, E>) -> T
    where
        T: Recover,
    {
        match r {
            Ok(v) => v,
            Err(e) => {
                self.error(e);
                T::recover()
            }
        }
    }

    pub fn recover_val<T>(&mut self, r: Result<T, E>, val: T) -> T {
        match r {
            Ok(v) => v,
            Err(e) => {
                self.error(e);
                val
            }
        }
    }

    pub fn finish<T>(self, output: Option<T>) -> PassResult<T, E, W> {
        PassResult::new(output, self.warnings, self.errors)
    }
}

macro_rules! impl_deref_context {
    ($t:ty, $e:ty, $w:ty) => {
        impl std::ops::Deref for $t {
            type Target = Context<$e, $w>;

            fn deref(&self) -> &Self::Target {
                &self.context
            }
        }

        impl std::ops::DerefMut for $t {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.context
            }
        }
    };
}
pub(crate) use impl_deref_context;

use crate::error::PassResult;

pub trait Recover {
    fn recover() -> Self;
}

impl<T: Default> Recover for T {
    fn recover() -> Self {
        Self::default()
    }
}
