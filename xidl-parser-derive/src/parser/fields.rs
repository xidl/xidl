use super::style::Style;

/// Ordered field collection with its syntactic shape.
#[derive(Debug)]
pub struct Fields<T> {
    /// Whether the fields are unit, tuple, or struct shaped.
    pub style: Style,
    /// Fields in source order.
    pub fields: Vec<T>,
}

impl<T> Fields<T> {
    /// Number of collected fields.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Whether no fields were collected.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Iterate over collected fields in source order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.fields.iter()
    }

    /// Build a collection from a `syn` field list.
    pub(crate) fn from_syn_fields(fields: &syn::Fields) -> syn::Result<Self>
    where
        T: FromSynField,
    {
        match fields {
            syn::Fields::Unit => Ok(Self {
                style: Style::Unit,
                fields: Vec::new(),
            }),
            syn::Fields::Named(named) => {
                let mut vec = Vec::new();
                for field in &named.named {
                    vec.push(T::from_syn_field(field)?);
                }
                Ok(Self {
                    style: Style::Struct,
                    fields: vec,
                })
            }
            syn::Fields::Unnamed(unnamed) => {
                let mut vec = Vec::new();
                for field in &unnamed.unnamed {
                    vec.push(T::from_syn_field(field)?);
                }
                Ok(Self {
                    style: Style::Tuple,
                    fields: vec,
                })
            }
        }
    }
}

/// Conversion from a `syn` field into a derive-model field.
pub(crate) trait FromSynField: Sized {
    /// Convert one `syn` field, reporting Parser derive input errors.
    fn from_syn_field(field: &syn::Field) -> syn::Result<Self>;
}
