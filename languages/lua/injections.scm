;; Injections for Lua code

;; LuaCats Doc comments injection
(((comment) @_emmyluadoc_comment
  (#match? @_emmyluadoc_comment "^---")) @injection.content
  (#set! injection.language "emmyluadoc"))

;; Add support for TODO, FIXME, HACK, etc with the "comment" extension
((comment) @injection.content
  (#set! injection.language "comment"))

;; LuaJIT FFI C code injection
((function_call
  name: [
    (identifier) @_cdef_identifier
    (_ _ (identifier) @_cdef_identifier)
  ]
  arguments: (arguments (string content: _ @injection.content
    (#set! injection.language "c"))))
  (#eq? @_cdef_identifier "cdef"))
