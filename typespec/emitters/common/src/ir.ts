/**
 * Intermediate Representation (IR) for JiraDb API
 *
 * This module defines the data structures that represent the parsed
 * TypeSpec definitions in a language-agnostic format.
 */

// ============================================================
// Type System
// ============================================================

/** Scalar type kinds */
export type ScalarKind =
  | "string"
  | "boolean"
  | "int32"
  | "int64"
  | "float32"
  | "float64"
  | "utcDateTime";

/** Type reference - either a scalar or a named model */
export interface TypeRef {
  kind: "scalar" | "model" | "array";
  /** For scalar types */
  scalar?: ScalarKind;
  /** For model/array types - the referenced model name */
  modelName?: string;
  /** For array types - the element type */
  elementType?: TypeRef;
}

// ============================================================
// Model Definitions
// ============================================================

/** A field within a model */
export interface IRField {
  /** Field name (camelCase) */
  name: string;
  /** Field type */
  type: TypeRef;
  /** Whether the field is optional */
  optional: boolean;
  /** Documentation comment */
  doc?: string;
}

/** A model (struct/interface) definition */
export interface IRModel {
  /** Model name (PascalCase) */
  name: string;
  /** Fields in the model */
  fields: IRField[];
  /** Documentation comment */
  doc?: string;
  /** Category for grouping (e.g., "entity", "request", "response", "config") */
  category?: string;
}

// ============================================================
// Operation Definitions
// ============================================================

/** An operation (RPC method) definition */
export interface IROperation {
  /** Operation name (camelCase, e.g., "list", "get", "enable") */
  name: string;
  /** Request model name */
  requestType: string;
  /** Response model name */
  responseType: string;
  /** Documentation comment */
  doc?: string;
}

/** A namespace containing operations */
export interface IRNamespace {
  /** Namespace name (PascalCase, e.g., "Config", "Projects") */
  name: string;
  /** Operations in this namespace */
  operations: IROperation[];
  /** Documentation comment */
  doc?: string;
}

// ============================================================
// Complete API Schema
// ============================================================

/** Complete API schema - the root of the IR */
export interface IRSchema {
  /** API title/name */
  title: string;
  /** API version */
  version: string;
  /** All model definitions */
  models: IRModel[];
  /** All namespace definitions (containing operations) */
  namespaces: IRNamespace[];
}

// ============================================================
// Helper Functions
// ============================================================

/** Create a scalar type reference */
export function scalarType(kind: ScalarKind): TypeRef {
  return { kind: "scalar", scalar: kind };
}

/** Create a model type reference */
export function modelType(name: string): TypeRef {
  return { kind: "model", modelName: name };
}

/** Create an array type reference */
export function arrayType(elementType: TypeRef): TypeRef {
  return { kind: "array", elementType };
}

/** Create an array of scalar type */
export function arrayOfScalar(kind: ScalarKind): TypeRef {
  return arrayType(scalarType(kind));
}

/** Create an array of model type */
export function arrayOfModel(name: string): TypeRef {
  return arrayType(modelType(name));
}
