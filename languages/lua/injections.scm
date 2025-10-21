;; Injections for Lua code

;; LuaCats Doc comments injection
;; Inject --- comments as LuaCats language
;; The emmyluadoc grammar now supports --- prefix, so we can directly inject
((comment
  (comment_content) @injection.content)
 (#match? @injection.content "^-")
 (#set! injection.language "emmyluadoc"))

;; LuaJIT FFI C code injection
((function_call
  name: [
    (identifier) @_cdef_identifier
    (_ _ (identifier) @_cdef_identifier)
  ]
  arguments: (arguments (string content: _ @injection.content
    (#set! injection.language "c"))))
  (#eq? @_cdef_identifier "cdef"))
