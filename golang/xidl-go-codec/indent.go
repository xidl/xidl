package json

import "bytes"

// Indent appends to dst an indented form of the JSON-encoded src.
func Indent(dst *bytes.Buffer, src []byte, prefix, indent string) error {
	var (
		depth   int
		inStr   bool
		escaped bool
	)

	writeIndent := func() {
		dst.WriteString(prefix)
		for i := 0; i < depth; i++ {
			dst.WriteString(indent)
		}
	}

	for i := 0; i < len(src); i++ {
		c := src[i]
		if inStr {
			dst.WriteByte(c)
			if escaped {
				escaped = false
			} else if c == '\\' {
				escaped = true
			} else if c == '"' {
				inStr = false
			}
			continue
		}

		switch c {
		case ' ', '\t', '\r', '\n':
			continue
		case '"':
			inStr = true
			dst.WriteByte(c)
		case '{', '[':
			dst.WriteByte(c)
			depth++
			dst.WriteByte('\n')
			writeIndent()
		case '}', ']':
			depth--
			dst.WriteByte('\n')
			writeIndent()
			dst.WriteByte(c)
		case ':':
			dst.WriteByte(':')
			dst.WriteByte(' ')
		case ',':
			dst.WriteByte(',')
			dst.WriteByte('\n')
			writeIndent()
		default:
			dst.WriteByte(c)
		}
	}
	return nil
}
