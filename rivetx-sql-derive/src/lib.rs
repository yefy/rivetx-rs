use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit, Meta, NestedMeta};

#[proc_macro_derive(FromSqlRow, attributes(db, attr, size))]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let mut cols = Vec::new();
    let mut field_indices = Vec::new();
    let mut sql_types = Vec::new();
    let mut field_idents = Vec::new();
    let mut fixed_attrs = Vec::new();

    let mut discard_auto_cols = Vec::new();
    let mut discard_auto_indices = Vec::new();
    let mut discard_auto_field_idents = Vec::new();

    let mut auto_col_names = Vec::new();
    let mut primary = quote! { None };
    let mut indices = Vec::new();
    let mut uniques = Vec::new();

    let mut sql_field_names = Vec::new();

    let mut from_row_fields = Vec::new();
    let mut from_sql_cell_fields = Vec::new();

    if let Data::Struct(data) = input.data {
        if let Fields::Named(fields) = data.fields {
            for (i, field) in fields.named.into_iter().enumerate() {
                let field_ident = field.ident.unwrap();
                let field_name = field_ident.to_string();

                // -------- db tag --------
                let mut col_name = field_name.clone();
                let mut is_ignore = false;
                for attr in &field.attrs {
                    if attr.path.is_ident("db") {
                        if let Ok(Meta::NameValue(nv)) = attr.parse_meta() {
                            if let Lit::Str(ls) = nv.lit {
                                let v = ls.value();
                                if v == "-" {
                                    is_ignore = true;
                                } else {
                                    col_name = v;
                                }
                            }
                        }
                    }
                }

                let sql_field_name = {
                    let lower = col_name.to_lowercase();
                    if let Some(pos) = lower.rfind(" as ") {
                        col_name[pos + 4..].trim()
                    } else if let Some(pos) = col_name.rfind('.') {
                        col_name[pos + 1..].trim()
                    } else {
                        col_name.trim()
                    }
                };

                if is_ignore {
                    from_row_fields.push(quote! {
                        #field_ident: Default::default()
                    });
                    from_sql_cell_fields.push(quote! {
                        #field_ident: Default::default()
                    });
                    continue;
                }

                // -------- size --------
                let mut size = "".to_string();
                for attr in &field.attrs {
                    if attr.path.is_ident("size") {
                        if let Ok(Meta::NameValue(nv)) = attr.parse_meta() {
                            if let Lit::Str(ls) = nv.lit {
                                size = ls.value();
                            }
                        }
                    }
                }

                let ty = &field.ty;
                let type_str = quote!(#ty).to_string().replace(" ", "");
                let sql_type = match type_str.as_str() {
                    "u64" | "u128" | "usize" | "ArcAtomicU64" => "BIGINT UNSIGNED".to_string(),
                    "u32" | "u16" | "u8" | "ArcAtomicU32" => "INT UNSIGNED".to_string(),
                    "i64" | "i128" | "isize" | "ArcAtomicI64" => "BIGINT".to_string(),
                    "i32" | "i16" | "i8" | "ArcAtomicI32" => "INT".to_string(),
                    "String" | "&str" | "ArcString" | "RivetxString" => {
                        // If size is a TEXT family type, use it directly
                        if size == "TINYTEXT"
                            || size == "TEXT"
                            || size == "MEDIUMTEXT"
                            || size == "LONGTEXT"
                        {
                            size.to_string()
                        } else {
                            if size.len() <= 0 {
                                size = "255".to_string();
                            }
                            match size.parse::<i32>() {
                                Ok(num) => format!("VARCHAR({})", num),
                                Err(e) => {
                                    return syn::Error::new_spanned(
                                        ty,
                                        format!("Invalid VARCHAR size '{}': {}", size, e),
                                    )
                                    .to_compile_error()
                                    .into();
                                }
                            }
                        }
                    }
                    "bool" => "TINYINT(1)".to_string(),
                    _ => {
                        if type_str.contains("NaiveDateTime") {
                            match size.parse::<i32>() {
                                Ok(num) => {
                                    if size.len() > 0 {
                                        format!("DATETIME({})", num)
                                    } else {
                                        "DATETIME".to_string()
                                    }
                                }
                                Err(_) => "DATETIME".to_string(),
                            }
                        } else {
                            return syn::Error::new_spanned(
                                ty,
                                format!("Unsupported SQL type mapping: {}", type_str),
                            )
                            .to_compile_error()
                            .into();
                        }
                    }
                };

                // -------- attr --------
                let mut is_auto = false;
                let mut fixed_attr = "".to_string();
                for attr in &field.attrs {
                    if attr.path.is_ident("attr") {
                        if let Ok(Meta::List(list)) = attr.parse_meta() {
                            for nested in list.nested {
                                match nested {
                                    NestedMeta::Meta(Meta::Path(p)) => {
                                        if p.is_ident("primary") {
                                            primary = quote! { Some(RString::from(#col_name)) };
                                        } else if p.is_ident("auto") {
                                            is_auto = true;
                                            auto_col_names.push(col_name.clone());
                                        }
                                    }
                                    NestedMeta::Lit(Lit::Str(ls)) => {
                                        fixed_attr = ls.value();
                                    }
                                    NestedMeta::Meta(Meta::NameValue(nv)) => {
                                        let key = nv.path.get_ident().unwrap().to_string();
                                        if let Lit::Str(ls) = nv.lit {
                                            let val = ls.value();
                                            if key == "index" {
                                                indices.push(quote! {
                                                    index_map.insert(RString::from(#val), RString::from(#col_name));
                                                });
                                            } else if key == "unique" {
                                                uniques.push(quote! {
                                                     unique_map.entry(RString::from(#val)).or_default().push(RString::from(#col_name));
                                                });
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                fixed_attrs.push(fixed_attr.clone());
                if fixed_attr.contains("DEFAULT") && fixed_attr.contains("CURRENT_TIMESTAMP") {
                    is_auto = true;
                }

                // -------- base --------
                cols.push(col_name.clone());
                field_indices.push(i);
                sql_types.push(sql_type);
                field_idents.push(field_ident.clone());

                // -------- Insert discard auto --------
                if !is_auto {
                    discard_auto_cols.push(col_name.clone());
                    discard_auto_indices.push(i);
                    discard_auto_field_idents.push(field_ident.clone());
                }

                // -------- FromSqlRow --------
                sql_field_names.push(sql_field_name.to_string());
                from_row_fields.push(quote! {
                    #field_ident: row.take(#sql_field_name)
                        .ok_or_else(|| mysql_common::FromRowError(row.clone()))?
                });
                from_sql_cell_fields.push(quote! {
                    #field_ident: {
                        let cell = ::rivetx_sql::take_sql_cell(cols, cells, #sql_field_name)?;
                        ::rivetx_sql::FromSqlCell::from_sql_cell(cell)?
                    }
                });
            }
        }
    }

    let expanded = quote! {

        // ========================
        // FromSqlRow
        // ========================

        impl FromSqlRow for #name {
            fn get_struct_meta() -> StructMeta {
                use std::collections::HashMap;
                type RString = rivetx_core::rivetx_string::RivetxString;

                let mut auto_col_map = HashMap::new();
                #(
                    auto_col_map.insert(RString::from(#auto_col_names), true);
                )*

                let mut index_map: HashMap<RString, RString> = HashMap::new();
                #( #indices )*

                let mut unique_map: HashMap<RString, Vec<RString>> = HashMap::new();
                #( #uniques )*

                StructMeta {
                    cols: vec![#( RString::from(#cols) ),*],
                    field_index: vec![#( #field_indices ),*],

                    sql_field_names: vec![#( RString::from(#sql_field_names) ),*],
                    sql_types: vec![#( RString::from(#sql_types) ),*],
                    fixed_attrs: vec![#( RString::from(#fixed_attrs) ),*],

                    discard_auto_cols: vec![#( RString::from(#discard_auto_cols) ),*],
                    discard_auto_field_index: vec![#( #discard_auto_indices ),*],

                    auto_col_map,
                    primary: #primary,
                    index_map,
                    unique_map,
                }
            }
        }

        // ========================
        // ToSqlValues
        // ========================
        impl ToSqlValues for #name {
            fn to_values_discard_auto(&self) -> Vec<::rivetx_sql::SqlCell> {
                vec![
                    #( ::rivetx_sql::SqlCell::from(self.#discard_auto_field_idents.clone()) ),*
                ]
            }

            fn to_values(&self) -> Vec<::rivetx_sql::SqlCell> {
                vec![
                    #( ::rivetx_sql::SqlCell::from(self.#field_idents.clone()) ),*
                ]
            }
        }

        impl ::rivetx_sql::FromSqlCells for #name {
            fn from_sql_cells(
                cols: &[::rivetx_core::rivetx_string::RivetxString],
                cells: &[::rivetx_sql::SqlCell],
            ) -> anyhow::Result<Self> {
                Ok(Self {
                    #( #from_sql_cell_fields, )*
                })
            }
        }

        // ========================
        // mysql FromRow (native consumers)
        // ========================
        #[cfg(feature = "native")]
        impl mysql_common::prelude::FromRow for #name {
            fn from_row_opt(mut row: mysql_common::Row)
                -> Result<Self, mysql_common::FromRowError>
            {
                Ok(Self {
                    #( #from_row_fields, )*
                })
            }
        }
    };

    TokenStream::from(expanded)
}
