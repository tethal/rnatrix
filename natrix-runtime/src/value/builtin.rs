macro_rules! define_builtins {
    ($($variant:ident => $name:literal, $param_count:expr);* $(;)?) => {
        #[repr(u8)]
        #[derive(Copy, Clone, Debug)]
        pub enum Builtin {
            $($variant),*
        }

        impl Builtin {
            pub const ALL: &'static [Builtin] = &[
                $(Builtin::$variant),*
            ];

            pub const fn name(self) -> &'static str {
                match self {
                    $(Builtin::$variant => $name),*
                }
            }

            pub const fn param_count(self) -> usize {
                match self {
                    $(Builtin::$variant => $param_count),*
                }
            }

            pub fn index(&self) -> usize {
                *self as u8 as usize
            }
        }
    };
}

define_builtins! {
    Float => "float", 1;
    Int => "int", 1;
    Len => "len", 1;
    Print => "print", 1;
    Str => "str", 1;
    Time => "time", 0;
}
