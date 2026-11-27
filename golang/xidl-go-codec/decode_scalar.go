package json

import (
	"fmt"
	"io"
	"strconv"
	"strings"
	"unicode/utf16"
	"unicode/utf8"
)

func (d *decoder) parseInt() (int64, error) {
	d.skipSpace()
	start := d.idx
	if d.idx < len(d.data) && d.data[d.idx] == '-' {
		d.idx++
	}
	for d.idx < len(d.data) && d.data[d.idx] >= '0' && d.data[d.idx] <= '9' {
		d.idx++
	}
	if start == d.idx {
		return 0, fmt.Errorf("json: expected integer at offset %d", start)
	}
	numStr := string(d.data[start:d.idx])
	if d.idx < len(d.data) &&
		(d.data[d.idx] == '.' || d.data[d.idx] == 'e' || d.data[d.idx] == 'E') {
		return 0, fmt.Errorf("json: expected integer at offset %d", start)
	}
	i, err := strconv.ParseInt(numStr, 10, 64)
	if err != nil {
		return 0, err
	}
	return i, nil
}

func (d *decoder) parseUint() (uint64, error) {
	d.skipSpace()
	start := d.idx
	for d.idx < len(d.data) && d.data[d.idx] >= '0' && d.data[d.idx] <= '9' {
		d.idx++
	}
	if start == d.idx {
		return 0, fmt.Errorf("json: expected integer at offset %d", start)
	}
	numStr := string(d.data[start:d.idx])
	if d.idx < len(d.data) &&
		(d.data[d.idx] == '.' || d.data[d.idx] == 'e' || d.data[d.idx] == 'E') {
		return 0, fmt.Errorf("json: expected integer at offset %d", start)
	}
	u, err := strconv.ParseUint(numStr, 10, 64)
	if err != nil {
		return 0, err
	}
	return u, nil
}

func (d *decoder) parseFloat() (float64, error) {
	d.skipSpace()
	start := d.idx
	if d.idx < len(d.data) && d.data[d.idx] == '-' {
		d.idx++
	}
	for d.idx < len(d.data) {
		c := d.data[d.idx]
		if (c >= '0' && c <= '9') || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
			d.idx++
		} else {
			break
		}
	}
	if start == d.idx {
		return 0, fmt.Errorf("json: expected number at offset %d", start)
	}
	numStr := string(d.data[start:d.idx])
	f, err := strconv.ParseFloat(numStr, 64)
	if err != nil {
		return 0, err
	}
	return f, nil
}

func (d *decoder) parseString() (string, error) {
	d.skipSpace()
	if d.idx >= len(d.data) || d.data[d.idx] != '"' {
		return "", fmt.Errorf("json: expected string start at offset %d", d.idx)
	}
	d.idx++
	var sb strings.Builder
	for d.idx < len(d.data) {
		c := d.data[d.idx]
		if c == '"' {
			d.idx++
			text := sb.String()
			if !utf8.ValidString(text) {
				return "", fmt.Errorf("json: invalid UTF-8 string")
			}
			return text, nil
		}
		if c == '\\' {
			if d.idx+1 >= len(d.data) {
				return "", io.ErrUnexpectedEOF
			}
			escaped := d.data[d.idx+1]
			d.idx += 2
			switch escaped {
			case '"', '\\', '/':
				sb.WriteByte(escaped)
			case 'b':
				sb.WriteByte('\b')
			case 'f':
				sb.WriteByte('\f')
			case 'n':
				sb.WriteByte('\n')
			case 'r':
				sb.WriteByte('\r')
			case 't':
				sb.WriteByte('\t')
			case 'u':
				if d.idx+4 > len(d.data) {
					return "", io.ErrUnexpectedEOF
				}
				hexStr := string(d.data[d.idx : d.idx+4])
				d.idx += 4
				r, err := strconv.ParseUint(hexStr, 16, 64)
				if err != nil {
					return "", fmt.Errorf("json: invalid unicode escape %q", hexStr)
				}
				if utf16.IsSurrogate(rune(r)) {
					if r < 0xD800 || r > 0xDBFF || d.idx+6 > len(d.data) || string(d.data[d.idx:d.idx+2]) != "\\u" {
						return "", fmt.Errorf("json: unpaired Unicode surrogate")
					}
					low, err := strconv.ParseUint(string(d.data[d.idx+2:d.idx+6]), 16, 16)
					if err != nil || low < 0xDC00 || low > 0xDFFF {
						return "", fmt.Errorf("json: unpaired Unicode surrogate")
					}
					d.idx += 6
					sb.WriteRune(utf16.DecodeRune(rune(r), rune(low)))
				} else {
					sb.WriteRune(rune(r))
				}
			default:
				return "", fmt.Errorf("json: invalid escape sequence \\%c", escaped)
			}
		} else {
			if c < 0x20 {
				return "", fmt.Errorf("json: unescaped control character in string")
			}
			sb.WriteByte(c)
			d.idx++
		}
	}
	return "", io.ErrUnexpectedEOF
}

func (d *decoder) parseBool() (bool, error) {
	d.skipSpace()
	if d.idx+4 <= len(d.data) && string(d.data[d.idx:d.idx+4]) == "true" {
		d.idx += 4
		return true, nil
	}
	if d.idx+5 <= len(d.data) && string(d.data[d.idx:d.idx+5]) == "false" {
		d.idx += 5
		return false, nil
	}
	return false, fmt.Errorf("json: expected boolean at offset %d", d.idx)
}
