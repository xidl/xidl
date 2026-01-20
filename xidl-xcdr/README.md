# xidl-xcdr

| Encoding Version | Encoding Name | used for types             | Description                                                      |
| ---------------- | ------------- | -------------------------- | ---------------------------------------------------------------- |
| NONE             | CDR           | FINAL and APPENDABLE       |                                                                  |
| XCDR1            | PLAIN_CDR     |                            | eXtend CDR so that it can handle optional members, bitmasks, map |
| XCDR1            | PL_CDR        |                            | same as RTPS Parameter List encoding to handle mutable types     |
| XCDR2            | PLAIN_CDR2    | FINAL                      |                                                                  |
| XCDR2            | DELIMITED_CDR | APPENDABLE                 |                                                                  |
| XCDR2            | PL_CDR2       | MUTABLE (aggregated types) |                                                                  |
