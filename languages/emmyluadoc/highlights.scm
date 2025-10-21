; Highlights query for EmmyLuaDoc
; Query file for syntax highlighting

; ============================================
; Lua comment prefix (Comment Prefix)
; ============================================

(comment_prefix) @comment

; ============================================
; Annotation keywords (Annotation Keywords)
; ============================================

; Match annotation tags using anonymous nodes
"@class" @keyword
"@field" @keyword
"@type" @keyword
"@param" @keyword
"@return" @keyword
"@generic" @keyword
"@vararg" @keyword
"@overload" @keyword
"@see" @keyword
"@alias" @keyword
"@enum" @keyword
"@module" @keyword
"@cast" @keyword
"@version" @keyword
"@diagnostic" @keyword
"@operator" @keyword

; Special annotation nodes (these are text nodes without @ prefix)
(deprecated_annotation) @keyword
(private_annotation) @keyword
(protected_annotation) @keyword
(public_annotation) @keyword
(package_annotation) @keyword
(async_annotation) @keyword
(nodiscard_annotation) @keyword
(meta_annotation) @keyword

; ============================================
; Type keywords (Type Keywords)
; ============================================

[
  "fun"
  "table"
] @keyword.type

; ============================================
; Basic types (Basic Types)
; ============================================

((identifier) @type.builtin
  (#match? @type.builtin "^(string|number|integer|boolean|table|function|thread|userdata|nil|any|unknown|self)$"))

; Custom types
(basic_type
  (identifier) @type)

; ============================================
; Class definitions (Class Definitions)
; ============================================

(class_annotation
  name: (identifier) @type.definition)

(class_annotation
  parent: (type_list
    (type
      (primary_type
        (basic_type
          (identifier) @type)))))

; ============================================
; Fields and parameters (Fields and Parameters)
; ============================================

(field_annotation
  name: (field_name) @variable.member)

(param_annotation
  name: (param_name) @variable.parameter)

(return_value
  name: (identifier)? @variable.parameter)

(param_def
  name: (identifier) @variable.parameter)

; ============================================
; Generics (Generics)
; ============================================

(generic_annotation
  name: (identifier) @type.parameter)

(generic_type
  base: (identifier) @type)

; ============================================
; Aliases and enums (Aliases and Enums)
; ============================================

(alias_annotation
  name: (identifier) @type.definition)

(enum_annotation
  name: (identifier) @type.definition)

; ============================================
; Visibility modifiers (Visibility Modifiers)
; ============================================

[
  "public"
  "private"
  "protected"
  "package"
] @keyword.modifier

; ============================================
; Operators (Operators)
; ============================================

[
  "call"
  "add" "sub" "mul" "div" "mod" "pow"
  "concat"
  "len"
  "eq" "lt" "le"
  "unm"
  "bnot" "band" "bor" "bxor" "shl" "shr"
] @operator

; ============================================
; Literals (Literals)
; ============================================

(string) @string

(number) @number

(boolean) @boolean

; ============================================
; Punctuation (Punctuation)
; ============================================

[
  ":"
  "|"
  ","
  "?"
] @punctuation.delimiter

; '|' in type continuation
(type_continuation
  "|" @punctuation.delimiter)

[
  "("
  ")"
  "["
  "]"
  "<"
  ">"
] @punctuation.bracket

; ============================================
; Tuple types (Tuple Types)
; ============================================

; Tuple brackets
(tuple_type
  "[" @punctuation.bracket
  "]" @punctuation.bracket)

; Commas in tuple elements
(tuple_elements
  "," @punctuation.delimiter)

; ============================================
; References and diagnostics (References and Diagnostics)
; ============================================

(see_annotation
  reference: (identifier) @variable)

(diagnostic_annotation
  action: [
    "disable"
    "enable"
    "disable-next-line"
    "disable-line"
  ] @keyword.directive)

(diagnostic_list
  (identifier) @constant)

; ============================================
; Module names (Module Names)
; ============================================

(module_annotation
  name: (string) @module)

; ============================================
; Version (Version)
; ============================================

(version_annotation
  version: [
    (identifier) @constant
    (string) @string
  ])

; ============================================
; Array type markers (Array Type Markers)
; ============================================

(array_type
  "[" @punctuation.bracket
  "]" @punctuation.bracket)

; ============================================
; Function types (Function Types)
; ============================================

(function_type
  "fun" @keyword.function
  ":" @punctuation.delimiter)

; ============================================
; Table types (Table Types)
; ============================================

(table_type
  "table" @type.builtin)

; Table literal types (Table Literal Types)
(table_literal_type
  "{" @punctuation.bracket
  "}" @punctuation.bracket)

; Named fields
(table_field
  name: (identifier) @property
  ":" @punctuation.delimiter
  type: (type_list
    (type
      (primary_type
        (basic_type
          (identifier) @type)))))

; Index fields
(table_field
  "[" @punctuation.bracket
  "]" @punctuation.bracket
  ":" @punctuation.delimiter)

; Commas in table fields
(table_literal_type
  "," @punctuation.delimiter)

; ============================================
; Generic parameters (Generic Parameters)
; ============================================

; Generic parameter definitions for @class and @alias
(generic_params
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(generic_params
  params: (identifier) @type.parameter)

; ============================================
; nil literal (nil Literal)
; ============================================

"nil" @constant.builtin
