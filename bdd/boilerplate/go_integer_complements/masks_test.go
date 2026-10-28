package api

import (
	"masks"
	"testing"
)

func TestUnsignedMaskBounds(t *testing.T) {
	cases := []struct {
		name string
		got  uint64
		want uint64
	}{
		{"remaining32", uint64(REMAINING32), 0},
		{"remaining64", REMAINING64, 0},
		{"maximum32", uint64(MAXIMUM32), 4294967295},
		{"maximum64", MAXIMUM64, 18446744073709551615},
		{"imported_width", REST, 4294967296},
		{"complement_width", WIDE, 18446744069414584320},
		{"included32", uint64(masks.ZERO32), 0},
		{"included64", masks.ALL64, 18446744073709551615},
		{"included_octet", uint64(masks.FLAGS), 255},
		{"root_scope", uint64(masks.PolicyROOT), 0},
		{"local_scope", uint64(masks.PolicyLOCAL), 2},
	}
	for _, c := range cases {
		if c.got != c.want {
			t.Errorf("integer mask %s: got %d, want %d", c.name, c.got, c.want)
		}
	}
}

func TestSignedMaskBounds(t *testing.T) {
	cases := []struct {
		name string
		got  int64
		want int64
	}{
		{"complement32", int64(COMPLEMENT32), 1},
		{"complement64", COMPLEMENT64, 1},
		{"maximum32", int64(MaximumSigned32), 2147483647},
		{"maximum64", MaximumSigned64, 9223372036854775807},
		{"double_complement", RESTORED, -2},
		{"included_negative", masks.ComplementNegative, 1},
	}
	for _, c := range cases {
		if c.got != c.want {
			t.Errorf("integer mask %s: got %d, want %d", c.name, c.got, c.want)
		}
	}
}

func TestPermissionDecisions(t *testing.T) {
	if ALLOWED&2 == 0 {
		t.Error("permission bit 2 must be allowed")
	}
	if ALLOWED&1 != 0 {
		t.Error("permission bit 1 must be denied")
	}
	if ALLOWED&DENIED != 0 || ALLOWED|DENIED != MAXIMUM32 {
		t.Error("allowed and denied permissions must partition all 32 bits")
	}
}

func TestLiteralContents(t *testing.T) {
	if masks.TEXT != "~keep!" || masks.SYMBOL != '~' {
		t.Error("complement syntax must preserve string and character contents")
	}
}
