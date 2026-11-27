package json

import (
	"bytes"
	"unicode/utf8"
)

func writeString(s string, buf *bytes.Buffer) {
	buf.WriteByte('"')
	start := 0
	for i := 0; i < len(s); {
		if b := s[i]; b < utf8.RuneSelf {
			if htmlSafeSet[b] {
				i++
				continue
			}
			if start < i {
				buf.WriteString(s[start:i])
			}
			switch b {
			case '\\', '"':
				buf.WriteByte('\\')
				buf.WriteByte(b)
			case '\n':
				buf.WriteByte('\\')
				buf.WriteByte('n')
			case '\r':
				buf.WriteByte('\\')
				buf.WriteByte('r')
			case '\t':
				buf.WriteByte('\\')
				buf.WriteByte('t')
			default:
				buf.WriteString(`\u00`)
				buf.WriteByte(hexTable[b>>4])
				buf.WriteByte(hexTable[b&0xF])
			}
			i++
			start = i
			continue
		}
		r, size := utf8.DecodeRuneInString(s[i:])
		if r == '\u2028' || r == '\u2029' {
			if start < i {
				buf.WriteString(s[start:i])
			}
			buf.WriteString(`\u202`)
			buf.WriteByte(hexTable[r&0xF])
			i += size
			start = i
			continue
		}
		i += size
	}
	if start < len(s) {
		buf.WriteString(s[start:])
	}
	buf.WriteByte('"')
}

var hexTable = "0123456789abcdef"

var htmlSafeSet = [utf8.RuneSelf]bool{
	' ':    true,
	'!':    true,
	'"':    false,
	'#':    true,
	'$':    true,
	'%':    true,
	'&':    false,
	'\'':   true,
	'(':    true,
	')':    true,
	'*':    true,
	'+':    true,
	',':    true,
	'-':    true,
	'.':    true,
	'/':    true,
	'0':    true,
	'1':    true,
	'2':    true,
	'3':    true,
	'4':    true,
	'5':    true,
	'6':    true,
	'7':    true,
	'8':    true,
	'9':    true,
	':':    true,
	';':    true,
	'<':    false,
	'=':    true,
	'>':    false,
	'?':    true,
	'@':    true,
	'A':    true,
	'B':    true,
	'C':    true,
	'D':    true,
	'E':    true,
	'F':    true,
	'G':    true,
	'H':    true,
	'I':    true,
	'J':    true,
	'K':    true,
	'L':    true,
	'M':    true,
	'N':    true,
	'O':    true,
	'P':    true,
	'Q':    true,
	'R':    true,
	'S':    true,
	'T':    true,
	'U':    true,
	'V':    true,
	'W':    true,
	'X':    true,
	'Y':    true,
	'Z':    true,
	'[':    true,
	'\\':   false,
	']':    true,
	'^':    true,
	'_':    true,
	'`':    true,
	'a':    true,
	'b':    true,
	'c':    true,
	'd':    true,
	'e':    true,
	'f':    true,
	'g':    true,
	'h':    true,
	'i':    true,
	'j':    true,
	'k':    true,
	'l':    true,
	'm':    true,
	'n':    true,
	'o':    true,
	'p':    true,
	'q':    true,
	'r':    true,
	's':    true,
	't':    true,
	'u':    true,
	'v':    true,
	'w':    true,
	'x':    true,
	'y':    true,
	'z':    true,
	'{':    true,
	'|':    true,
	'}':    true,
	'~':    true,
	'\x7f': true,
}
