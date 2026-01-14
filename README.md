# xidl

xidl is a workspace for parsing IDL and generating code. It contains:

- xidl-parser: converts IDL into a typed AST and then HIR.
- xidlc (idlc): CLI that generates C code and xcdr helpers from HIR.
- xidl-xcdr: a small visitor-style serialization library (CDR/PLCDR/CDR3) with C bindings.
