import * as yup from "yup";

export function getDefinition(name: string, property: any, definitions: any) {
  if (property.$ref) {
    const ref = property.$ref.replace("#/definitions/", "");
    return definitions[ref];
  } else if (property.allOf) {
    const ref = property.allOf[0].$ref.replace("#/definitions/", "");
    return definitions[ref];
  } else if (property.anyOf) {
    const ref = property.anyOf[0].$ref.replace("#/definitions/", "");
    return definitions[ref];
  } else if (property.propertyNames) {
    throw new Error(`${name} is not supported.`);
  } else {
    return property;
  }
}
export function createYupSchema(schema: any, definitions: any) {
  return _createYupSchema(schema, schema, definitions, 0);
}
function _createYupSchema(
  property: any,
  definition: any,
  definitions: any,
  nest_count: number,
): any {
  if (nest_count > 100) return;
  if (definition.type === "object") {
    const defs: any = {};
    const required = definition.required || [];
    for (const name in definition.properties) {
      const _property = definition.properties[name];
      const _definition = getDefinition(name, _property, definitions);
      let def = _createYupSchema(
        _property,
        _definition,
        definitions,
        nest_count + 1,
      );
      if (def) {
        if (required.includes(name)) {
          def = def.required();
        }
        defs[name] = def;
      }
    }
    return yup.object(defs);
  }
  let def = null;
  if (definition.type === "string") {
    def = yup.string();
    if (definition.pattern) {
      def = def.matches(
        new RegExp(definition.pattern, "u"),
        "${label} is not valid",
      );
    }
  } else if (definition.type === "integer") {
    def = yup
      .number()
      .integer()
      .transform((value, _originalValue) =>
        Number.isNaN(value) ? null : value,
      );
    if (definition.minimum) {
      def = def.min(definition.minimum);
    }
  } else if (definition.type === "boolean") {
    def = yup.boolean();
  } else if (definition.type === "array") {
    const items = definition.items.$ref
      ? definitions[definition.items.$ref.replace("#/definitions/", "")]
      : definition.items;
    if (items.type === "object") {
      def = yup
        .array()
        .of(_createYupSchema(items, items, definitions, nest_count));
      if (items.properties.name) {
        def = def.test("unique", "${path} must be unique", (list: any) => {
          return (
            !list || list.length === new Set(list.map((i: any) => i.name)).size
          );
        });
      }
    } else if (items.type === "string") {
      let d = yup.string().required();
      if (property.title) {
        d = d.label(property.title);
      }
      if (items.pattern) {
        d = d.matches(
          new RegExp(items.pattern, "u"),
          "${label} is not valid",
        );
      }
      def = yup.array().of(d);
    }
    if (def && definition.minItems !== undefined) {
      def = def.min(definition.minItems);
    }
    if (def && definition.maxItems !== undefined) {
      def = def.max(definition.maxItems);
    }
  } else if (definition.oneOf) {
    if ("enum" in definition.oneOf[0]) {
      const values = [];
      for (const row of definition.oneOf) {
        for (const e of row.enum) {
          values.push(e);
        }
      }
      def = yup.mixed().oneOf(values);
    }
  } else if (definition.enum) {
    def = yup
      .mixed()
      .oneOf(
        definition.enum.map((val: any) =>
          typeof val === "string" ? val : val.const,
        ),
      );
  }
  if (!def) {
    throw new Error("unsupported");
  }
  if (property.title) {
    def = def.label(property.title);
  }
  if (property.nullable) {
    def = def.nullable();
  }
  return def;
}
