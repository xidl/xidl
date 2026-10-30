package xidlgo

import (
	"fmt"
	json "github.com/xidl/xidl/golang/xidl-go-codec"
	"unicode/utf8"
)

// Char is one Unicode scalar, shared by IDL char and wchar.
type Char rune

// MarshalText encodes a character without changing numeric int32 values.
func (c Char) MarshalText() ([]byte, error) {
	if !utf8.ValidRune(rune(c)) {
		return nil, fmt.Errorf("xidl char: invalid Unicode scalar %U", rune(c))
	}
	return []byte(string(rune(c))), nil
}

// UnmarshalText accepts exactly one Unicode scalar.
func (c *Char) UnmarshalText(data []byte) error {
	if !utf8.Valid(data) || utf8.RuneCount(data) != 1 {
		return fmt.Errorf("xidl char: expected one Unicode scalar")
	}
	value, _ := utf8.DecodeRune(data)
	*c = Char(value)
	return nil
}

// MarshalJSON uses the character's string representation.
func (c Char) MarshalJSON() ([]byte, error) {
	data, err := c.MarshalText()
	if err != nil {
		return nil, err
	}
	return json.Marshal(string(data))
}

// UnmarshalJSON rejects numbers and strings containing multiple scalars.
func (c *Char) UnmarshalJSON(data []byte) error {
	var text string
	if err := json.Unmarshal(data, &text); err != nil {
		return fmt.Errorf("xidl char: %w", err)
	}
	return c.UnmarshalText([]byte(text))
}
