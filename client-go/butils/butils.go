package butils

import (
	"encoding/binary"
)

func FromUint16(i uint16) []byte {
	b := make([]byte, 2)
	binary.BigEndian.PutUint16(b, i)
	return b
}

func ToUint16(b []byte) uint16 {
	return binary.BigEndian.Uint16(b)
}

func FromUint64(i uint64) []byte {
	b := make([]byte, 8)
	binary.BigEndian.PutUint64(b, i)
	return b
}

func ToUint64(b []byte) uint64 {
	return binary.BigEndian.Uint64(b)
}
