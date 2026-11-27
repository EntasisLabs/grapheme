//! Data-only element catalog (RFC-0007).
//!
//! A catalog names elements, props, prop types, events and the payload
//! params those events supply. The compiler checks a view against that data.
//! It does not know how an element is drawn. The default catalog is the five
//! built-ins (`Column`, `Row`, `Text`, `Button`, `TextField`). A host such as
//! GraphemeUI passes its own document through
//! [`CompileOptions::element_catalog`](crate::pipeline::CompileOptions::element_catalog).

use std::sync::LazyLock;

use serde_json::{Map, Value as JsonValue};

use crate::error::CompilerError;

/// Name of the catalog used when a compile does not pass one.
pub const BUILTIN_CATALOG_NAME: &str = "grapheme/builtins";

/// Prop types a catalog may declare.
///
/// `Text` accepts any scalar and is not checked against a store field's type.
/// The others must match the expression's static type when that type is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropType {
    /// Any scalar, rendered as text.
    Text,
    /// `String`.
    String,
    /// `Int`.
    Int,
    /// `Float`. An `Int` literal is not widened to `Float` for props.
    Float,
    /// `Bool`.
    Bool,
}

impl PropType {
    /// Catalog spelling of this type.
    pub fn as_str(self) -> &'static str {
        match self {
            PropType::Text => "Text",
            PropType::String => "String",
            PropType::Int => "Int",
            PropType::Float => "Float",
            PropType::Bool => "Bool",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "Text" => Some(PropType::Text),
            "String" => Some(PropType::String),
            "Int" => Some(PropType::Int),
            "Float" => Some(PropType::Float),
            "Bool" => Some(PropType::Bool),
            _ => None,
        }
    }
}

/// One parameter an event payload supplies at invoke time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogParam {
    /// Parameter name, without `$`.
    pub name: String,
    /// `String`, `Int`, `Float` or `Bool`.
    pub ty: String,
}

/// An event a catalog element can bind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEvent {
    /// Event name, such as `click` or `change`.
    pub name: String,
    /// Params the host fills in. Empty for `Button` `click`.
    pub payload: Vec<CatalogParam>,
}

/// A named prop and the type of its expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogProp {
    /// Prop name.
    pub name: String,
    /// Declared type.
    pub ty: PropType,
}

/// One element in a catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogElement {
    /// Element name used in view source (`Button`, `Divider`, …).
    pub name: String,
    /// View IR role (`button`, `text`, `group`, …).
    pub role: String,
    /// Accepts positional content, stored as the `content` prop.
    pub content: bool,
    /// May contain child elements.
    pub children: bool,
    /// Named props. Positional content uses the `content` prop's type when
    /// that prop is declared, and [`PropType::Text`] otherwise.
    pub props: Vec<CatalogProp>,
    /// Events the element may bind.
    pub events: Vec<CatalogEvent>,
}

impl CatalogElement {
    /// Prop declared under `name`.
    pub fn prop(&self, name: &str) -> Option<&CatalogProp> {
        self.props.iter().find(|prop| prop.name == name)
    }

    /// Event declared under `name`.
    pub fn event(&self, name: &str) -> Option<&CatalogEvent> {
        self.events.iter().find(|event| event.name == name)
    }

    /// The element declares at least one event.
    pub fn interactive(&self) -> bool {
        !self.events.is_empty()
    }
}

/// Elements a compile checks view trees against.
///
/// Load one with [`ElementCatalog::parse`]. [`ElementCatalog::builtin`] is the
/// catalog used when no document is passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementCatalog {
    name: String,
    elements: Vec<CatalogElement>,
}

impl Default for ElementCatalog {
    fn default() -> Self {
        Self::builtin()
    }
}

impl ElementCatalog {
    /// The five built-in elements, named [`BUILTIN_CATALOG_NAME`].
    pub fn builtin() -> Self {
        static BUILTIN: LazyLock<ElementCatalog> = LazyLock::new(|| {
            let catalog = ElementCatalog::parse(include_str!("builtin_catalog.json"))
                .expect("built-in element catalog is valid");
            assert_eq!(
                catalog.name(),
                BUILTIN_CATALOG_NAME,
                "built-in catalog document must be named {BUILTIN_CATALOG_NAME}"
            );
            catalog
        });
        BUILTIN.clone()
    }

    /// Parse a catalog document.
    ///
    /// ```json
    /// { "$catalog": "grapheme-ui/v1", "elements": [ { "name": "Text", "role": "text", "content": true, "children": false, "props": [ { "name": "content", "type": "Text" } ], "events": [] } ] }
    /// ```
    ///
    /// `$catalog` is the name unknown-element and unknown-prop errors use.
    /// Prop types are `Text`, `String`, `Int`, `Float` and `Bool`. Payload
    /// param types are `String`, `Int`, `Float` and `Bool`.
    pub fn parse(json: &str) -> Result<Self, CompilerError> {
        let value: JsonValue = serde_json::from_str(json)
            .map_err(|err| invalid(format!("catalog document is not JSON: {err}")))?;
        Self::from_value(&value)
    }

    /// Name recorded in the document (`$catalog`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Element declared under `name`.
    pub fn element(&self, name: &str) -> Option<&CatalogElement> {
        self.elements.iter().find(|element| element.name == name)
    }

    /// Elements in document order.
    pub fn elements(&self) -> &[CatalogElement] {
        &self.elements
    }

    /// Element names in document order, comma-separated.
    pub fn element_names(&self) -> String {
        self.elements
            .iter()
            .map(|element| element.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn from_value(value: &JsonValue) -> Result<Self, CompilerError> {
        let obj = expect_object(value, "element catalog")?;
        let name_value = obj
            .get("$catalog")
            .ok_or_else(|| invalid("element catalog is missing string \"$catalog\"".to_string()))?;
        let name = expect_name(name_value, "element catalog \"$catalog\"")?;
        Self::from_object_with_name(value, &name)
    }

    fn from_object_with_name(value: &JsonValue, name: &str) -> Result<Self, CompilerError> {
        let obj = expect_object(value, "element catalog")?;
        for key in obj.keys() {
            if key != "$catalog" && key != "elements" {
                return Err(invalid_named(name, format!("has unknown field '{key}'")));
            }
        }
        let elements_value = obj
            .get("elements")
            .ok_or_else(|| invalid_named(name, "is missing \"elements\"".to_string()))?;
        let elements = elements_value
            .as_array()
            .ok_or_else(|| invalid_named(name, "\"elements\" must be an array".to_string()))?;
        let mut parsed = Vec::with_capacity(elements.len());
        for (index, element) in elements.iter().enumerate() {
            parsed.push(parse_element(name, index, element)?);
        }
        let mut seen = Vec::new();
        for element in &parsed {
            if seen.iter().any(|name: &String| name == &element.name) {
                return Err(invalid_named(
                    name,
                    format!("declares element '{}' more than once", element.name),
                ));
            }
            seen.push(element.name.clone());
        }
        Ok(Self {
            name: name.to_string(),
            elements: parsed,
        })
    }
}

fn parse_element(
    catalog: &str,
    index: usize,
    value: &JsonValue,
) -> Result<CatalogElement, CompilerError> {
    let obj = expect_object(
        value,
        &format!("element catalog '{catalog}' element {index}"),
    )?;
    for key in obj.keys() {
        if !matches!(
            key.as_str(),
            "name" | "role" | "content" | "children" | "props" | "events"
        ) {
            return Err(invalid_named(
                catalog,
                format!("element {index} has unknown field '{key}'"),
            ));
        }
    }
    let name = expect_name(
        obj.get("name").ok_or_else(|| {
            invalid_named(
                catalog,
                format!("element {index} is missing string \"name\""),
            )
        })?,
        &format!("element catalog '{catalog}' element {index} \"name\""),
    )?;
    if name == "if" || name == "for" {
        return Err(invalid_named(
            catalog,
            format!("element '{name}' is reserved for view control flow"),
        ));
    }
    let where_ = format!("element '{name}'");
    let role = expect_name(
        obj.get("role").ok_or_else(|| {
            invalid_named(catalog, format!("{where_} is missing string \"role\""))
        })?,
        &format!("element catalog '{catalog}' {where_} \"role\""),
    )?;
    let content = expect_bool(obj.get("content"), catalog, &where_, "content")?;
    let children = expect_bool(obj.get("children"), catalog, &where_, "children")?;
    let props = parse_props(catalog, &where_, obj.get("props"))?;
    let events = parse_events(catalog, &where_, obj.get("events"))?;
    Ok(CatalogElement {
        name,
        role,
        content,
        children,
        props,
        events,
    })
}

fn parse_props(
    catalog: &str,
    element: &str,
    value: Option<&JsonValue>,
) -> Result<Vec<CatalogProp>, CompilerError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| invalid_named(catalog, format!("{element} \"props\" must be an array")))?;
    let mut props = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let obj = expect_object(
            item,
            &format!("element catalog '{catalog}' {element} prop {index}"),
        )?;
        for key in obj.keys() {
            if key != "name" && key != "type" {
                return Err(invalid_named(
                    catalog,
                    format!("{element} prop {index} has unknown field '{key}'"),
                ));
            }
        }
        let name = expect_name(
            obj.get("name").ok_or_else(|| {
                invalid_named(
                    catalog,
                    format!("{element} prop {index} is missing string \"name\""),
                )
            })?,
            &format!("element catalog '{catalog}' {element} prop {index} \"name\""),
        )?;
        if props.iter().any(|prop: &CatalogProp| prop.name == name) {
            return Err(invalid_named(
                catalog,
                format!("{element} declares prop '{name}' more than once"),
            ));
        }
        let ty_raw = expect_string(
            obj.get("type").ok_or_else(|| {
                invalid_named(
                    catalog,
                    format!("{element} prop '{name}' is missing string \"type\""),
                )
            })?,
            &format!("element catalog '{catalog}' {element} prop '{name}' \"type\""),
        )?;
        let Some(ty) = PropType::parse(&ty_raw) else {
            return Err(invalid_named(
                catalog,
                format!(
                    "{element} prop '{name}' has unknown type '{ty_raw}'; expected Text, String, Int, Float or Bool"
                ),
            ));
        };
        props.push(CatalogProp { name, ty });
    }
    Ok(props)
}

fn parse_events(
    catalog: &str,
    element: &str,
    value: Option<&JsonValue>,
) -> Result<Vec<CatalogEvent>, CompilerError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| invalid_named(catalog, format!("{element} \"events\" must be an array")))?;
    let mut events = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let obj = expect_object(
            item,
            &format!("element catalog '{catalog}' {element} event {index}"),
        )?;
        for key in obj.keys() {
            if key != "name" && key != "payload" {
                return Err(invalid_named(
                    catalog,
                    format!("{element} event {index} has unknown field '{key}'"),
                ));
            }
        }
        let name = expect_name(
            obj.get("name").ok_or_else(|| {
                invalid_named(
                    catalog,
                    format!("{element} event {index} is missing string \"name\""),
                )
            })?,
            &format!("element catalog '{catalog}' {element} event {index} \"name\""),
        )?;
        if events.iter().any(|event: &CatalogEvent| event.name == name) {
            return Err(invalid_named(
                catalog,
                format!("{element} declares event '{name}' more than once"),
            ));
        }
        let payload = parse_payload(catalog, element, &name, obj.get("payload"))?;
        events.push(CatalogEvent { name, payload });
    }
    Ok(events)
}

fn parse_payload(
    catalog: &str,
    element: &str,
    event: &str,
    value: Option<&JsonValue>,
) -> Result<Vec<CatalogParam>, CompilerError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value.as_array().ok_or_else(|| {
        invalid_named(
            catalog,
            format!("{element} event '{event}' \"payload\" must be an array"),
        )
    })?;
    let mut params = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let obj = expect_object(
            item,
            &format!("element catalog '{catalog}' {element} event '{event}' payload {index}"),
        )?;
        for key in obj.keys() {
            if key != "name" && key != "type" {
                return Err(invalid_named(
                    catalog,
                    format!("{element} event '{event}' payload {index} has unknown field '{key}'"),
                ));
            }
        }
        let name = expect_name(
            obj.get("name").ok_or_else(|| {
                invalid_named(
                    catalog,
                    format!("{element} event '{event}' payload {index} is missing string \"name\""),
                )
            })?,
            &format!(
                "element catalog '{catalog}' {element} event '{event}' payload {index} \"name\""
            ),
        )?;
        if params.iter().any(|param: &CatalogParam| param.name == name) {
            return Err(invalid_named(
                catalog,
                format!("{element} event '{event}' declares payload param '{name}' more than once"),
            ));
        }
        let ty_raw = expect_string(
            obj.get("type").ok_or_else(|| {
                invalid_named(
                    catalog,
                    format!(
                        "{element} event '{event}' payload param '{name}' is missing string \"type\""
                    ),
                )
            })?,
            &format!(
                "element catalog '{catalog}' {element} event '{event}' payload param '{name}' \"type\""
            ),
        )?;
        if !matches!(ty_raw.as_str(), "String" | "Int" | "Float" | "Bool") {
            return Err(invalid_named(
                catalog,
                format!(
                    "{element} event '{event}' payload param '{name}' has unknown type '{ty_raw}'; expected String, Int, Float or Bool"
                ),
            ));
        }
        params.push(CatalogParam { name, ty: ty_raw });
    }
    Ok(params)
}

fn expect_bool(
    value: Option<&JsonValue>,
    catalog: &str,
    element: &str,
    field: &str,
) -> Result<bool, CompilerError> {
    match value {
        None => Ok(false),
        Some(JsonValue::Bool(flag)) => Ok(*flag),
        Some(_) => Err(invalid_named(
            catalog,
            format!("{element} \"{field}\" must be a boolean"),
        )),
    }
}

fn expect_object<'a>(
    value: &'a JsonValue,
    what: &str,
) -> Result<&'a Map<String, JsonValue>, CompilerError> {
    value
        .as_object()
        .ok_or_else(|| invalid(format!("{what} must be a JSON object")))
}

fn expect_string(value: &JsonValue, what: &str) -> Result<String, CompilerError> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| invalid(format!("{what} must be a string")))
}

fn expect_name(value: &JsonValue, what: &str) -> Result<String, CompilerError> {
    let name = expect_string(value, what)?;
    if name.is_empty() || name != name.trim() {
        return Err(invalid(format!(
            "{what} must be a non-empty name with no surrounding whitespace"
        )));
    }
    Ok(name)
}

fn invalid(message: String) -> CompilerError {
    CompilerError::type_error(format!("invalid element catalog: {message}"))
}

fn invalid_named(catalog: &str, message: String) -> CompilerError {
    CompilerError::type_error(format!("invalid element catalog '{catalog}': {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_catalog_is_the_five_elements() {
        let catalog = ElementCatalog::builtin();
        assert_eq!(catalog.name(), BUILTIN_CATALOG_NAME);
        assert_eq!(
            catalog
                .elements()
                .iter()
                .map(|element| element.name.as_str())
                .collect::<Vec<_>>(),
            ["Column", "Row", "Text", "Button", "TextField"]
        );
        let button = catalog.element("Button").unwrap();
        assert!(button.interactive());
        assert!(button.children);
        assert_eq!(button.prop("enabled").unwrap().ty, PropType::Bool);
        let field = catalog.element("TextField").unwrap();
        assert_eq!(
            field.event("change").unwrap().payload[0].ty.as_str(),
            "String"
        );
    }

    #[test]
    fn rejects_unknown_prop_types_events_and_payload_params() {
        let err = ElementCatalog::parse(
            r#"{"$catalog":"sample","elements":[{"name":"Box","role":"group","props":[{"name":"tint","type":"Colour"}]}]}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("catalog 'sample'") && err.contains("unknown type 'Colour'"),
            "{err}"
        );

        let err = ElementCatalog::parse(
            r#"{"$catalog":"sample","elements":[{"name":"Box","role":"group","events":[{"name":"spin","payload":[{"name":"value","type":"Text"}]}]}]}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("payload param 'value'") && err.contains("unknown type 'Text'"),
            "{err}"
        );

        let err = ElementCatalog::parse(
            r#"{"$catalog":"sample","elements":[{"name":"Box","role":"group"},{"name":"Box","role":"group"}]}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("declares element 'Box' more than once"),
            "{err}"
        );
    }
}
