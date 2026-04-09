macro_rules! build_fn {
    ( $vis:vis $name:ident $( $field:ident ),+ ) => {
        #[doc = concat!("Constructs a new [`", stringify!($name),"`] taking all fields values from this object.")]
        $vis fn build(self) -> $name {
            $name {
                $(
                    $field: self.$field,
                )*
            }
        }
    };
}
pub(crate) use build_fn;

macro_rules! set_value {
    ( $self:ident $field:ident $value:expr ) => {{
        $self.$field = $value;
        $self
    }};
}
pub(crate) use set_value;

macro_rules! new {
    ( $vis:vis $name:ident ) => {
        #[doc = concat!("Constructs a new [`", stringify!($name),"`].")]
        $vis fn new() -> $name {
            $name {
                ..Default::default()
            }
        }
    };
}
pub(crate) use new;

macro_rules! from {
    ( $name:ident $to:ident $( $field:ident ),+ ) => {
        impl From<$name> for $to {
            fn from(value: $name) -> Self {
                Self {
                    $( $field: value.$field, )*
                }
            }
        }

        impl From<$to> for $name {
            fn from(value: $to) -> Self {
                value.build()
            }
        }
    };
}
pub(crate) use from;

macro_rules! builder {
    ( $( #[$builder_meta:meta] )* $builder_name:ident; $(#[$meta:meta])* $vis:vis $key:ident $name:ident $( $tt:tt )* ) => {
        builder!( @type_impl $builder_name $( #[$meta] )* $vis $key $name $( $tt )* );
        builder!( @builder_impl $( #[$builder_meta] )* $builder_name $( #[$meta] )* $vis $key $name $( $tt )* );
    };

    ( @type_impl $builder_name:ident $( #[$meta:meta] )* $vis:vis $key:ident $name:ident
        { $( $( #[$field_meta:meta] )* $field_vis:vis $field:ident: $field_ty:ty, )* }
    ) => {
        $( #[$meta] )*
        $vis $key $name {
            $( $( #[$field_meta] )* $field_vis $field: $field_ty, )*
        }

        impl $name {
            #[doc = concat!("Construct a new ", stringify!($builder_name), ".")]
            #[doc = ""]
            #[doc = concat!("This is effectively same as calling [`", stringify!($builder_name), "::new`]")]
            $vis fn builder() -> $builder_name {
                $builder_name::new()
            }
        }
    };

    ( @builder_impl $( #[$builder_meta:meta] )* $builder_name:ident $( #[$meta:meta] )* $vis:vis $key:ident $name:ident
        { $( $( #[$field_meta:meta] )* $field_vis:vis $field:ident: $field_ty:ty, )* }
    ) => {
        #[doc = concat!("Builder for [`", stringify!($name),
            "`] with chainable configuration methods to create a new [`", stringify!($name) , "`].")]
        $( #[$builder_meta] )*
        $vis $key $builder_name {
            $( $field: $field_ty, )*
        }

        impl Default for $builder_name {
            fn default() -> Self {
                let meta_default: $name = $name::default();
                Self {
                    $( $field: meta_default.$field, )*
                }
            }
        }

        impl $builder_name {
            crate::openapi::new!($vis $builder_name);
            crate::openapi::build_fn!($vis $name $( $field ),* );
        }

        crate::openapi::from!($name $builder_name $( $field ),* );
    };
}
pub(crate) use builder;
