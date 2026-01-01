/**
 * TypeSpec Parser
 *
 * Parses TypeSpec definitions and converts them to the IR format.
 */

import {
  Model,
  Namespace,
  Operation,
  Program,
  Type,
  getDoc,
  navigateProgram,
} from "@typespec/compiler";

import {
  IRSchema,
  IRModel,
  IRField,
  IRNamespace,
  IROperation,
  TypeRef,
  ScalarKind,
} from "./ir.js";

/** API namespace names to extract operations from */
const API_NAMESPACES = [
  "Config",
  "Projects",
  "Sync",
  "Issues",
  "Metadata",
  "Embeddings",
  "Reports",
];

/**
 * Parse a TypeSpec program into IR schema
 */
export function parseProgram(program: Program): IRSchema {
  const models: IRModel[] = [];
  const namespaces: IRNamespace[] = [];

  navigateProgram(program, {
    model(model: Model) {
      // Skip TypeSpec built-in types
      if (model.namespace?.name === "TypeSpec") {
        return;
      }
      // Skip Array type itself
      if (model.name === "Array") {
        return;
      }

      const irModel = parseModel(model, program);
      if (irModel) {
        models.push(irModel);
      }
    },
    namespace(namespace: Namespace) {
      if (API_NAMESPACES.includes(namespace.name)) {
        const irNamespace = parseNamespace(namespace, program);
        if (irNamespace) {
          namespaces.push(irNamespace);
        }
      }
    },
  });

  return {
    title: "JiraDb API",
    version: "1.0.0",
    models,
    namespaces,
  };
}

/**
 * Parse a TypeSpec Model into IR Model
 */
function parseModel(model: Model, program: Program): IRModel | null {
  const fields: IRField[] = [];

  for (const [fieldName, field] of model.properties) {
    const typeRef = parseType(field.type);
    if (typeRef) {
      fields.push({
        name: fieldName,
        type: typeRef,
        optional: field.optional,
        doc: getDocString(program, field),
      });
    }
  }

  // Determine category based on model name
  let category: string | undefined;
  if (model.name.endsWith("Request")) {
    category = "request";
  } else if (model.name.endsWith("Response")) {
    category = "response";
  } else if (model.name.endsWith("Config")) {
    category = "config";
  } else {
    category = "entity";
  }

  return {
    name: model.name,
    fields,
    doc: getDocString(program, model),
    category,
  };
}

/**
 * Parse a TypeSpec Namespace into IR Namespace
 */
function parseNamespace(
  namespace: Namespace,
  program: Program
): IRNamespace | null {
  const operations: IROperation[] = [];

  for (const [opName, op] of namespace.operations) {
    const irOp = parseOperation(opName, op, namespace.name, program);
    if (irOp) {
      operations.push(irOp);
    }
  }

  if (operations.length === 0) {
    return null;
  }

  return {
    name: namespace.name,
    operations,
    doc: getDocString(program, namespace),
  };
}

/**
 * Parse a TypeSpec Operation into IR Operation
 */
function parseOperation(
  name: string,
  op: Operation,
  namespaceName: string,
  program: Program
): IROperation | null {
  // Derive request/response type names from namespace and operation name
  const baseName = capitalize(name);

  // Handle naming conventions
  let singular: string;
  if (namespaceName === "Config") {
    singular = "Config";
  } else if (namespaceName.endsWith("s")) {
    // Projects -> Project, Issues -> Issue, etc.
    singular = namespaceName.slice(0, -1);
  } else {
    singular = namespaceName;
  }

  const requestType = `${singular}${baseName}Request`;
  const responseType = `${singular}${baseName}Response`;

  return {
    name,
    requestType,
    responseType,
    doc: getDocString(program, op),
  };
}

/**
 * Parse a TypeSpec Type into IR TypeRef
 */
function parseType(type: Type): TypeRef | null {
  switch (type.kind) {
    case "Scalar":
      return parseScalarType(type.name);

    case "Model":
      if (type.name === "Array") {
        // Handle array types
        const elementType = type.templateMapper?.args?.[0];
        if (elementType) {
          const elementRef = parseType(elementType as Type);
          if (elementRef) {
            return {
              kind: "array",
              elementType: elementRef,
            };
          }
        }
        return null;
      }
      return {
        kind: "model",
        modelName: type.name,
      };

    case "Union":
      // For now, treat unions as unknown
      return {
        kind: "scalar",
        scalar: "string",
      };

    default:
      return null;
  }
}

/**
 * Parse a scalar type name to ScalarKind
 */
function parseScalarType(name: string): TypeRef | null {
  const scalarMap: Record<string, ScalarKind> = {
    string: "string",
    boolean: "boolean",
    int32: "int32",
    int64: "int64",
    float32: "float32",
    float64: "float64",
    float: "float64",
    utcDateTime: "utcDateTime",
  };

  const scalar = scalarMap[name];
  if (scalar) {
    return {
      kind: "scalar",
      scalar,
    };
  }
  return null;
}

/**
 * Get documentation string from a TypeSpec element
 */
function getDocString(program: Program, element: unknown): string | undefined {
  try {
    const doc = getDoc(program, element as Type);
    return doc || undefined;
  } catch {
    return undefined;
  }
}

/**
 * Capitalize first letter
 */
function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}
