package mpc

import (
	"bytes"
	"testing"
)

func TestTranspose(t *testing.T) {
	input := []byte{
		0b01010110, 0b10000100, 0b01111010, 0b11111110,
		0b10010111, 0b10011001, 0b01110011, 0b10011011,
		0b00111101, 0b10010110, 0b01110111, 0b10010111,
		0b00101010, 0b00111011, 0b01011000, 0b11101111,
		0b01100000, 0b10011011, 0b10100111, 0b10000011,
		0b00110010, 0b01000010, 0b10001111, 0b10000101,
		0b11101101, 0b00100101, 0b00110100, 0b11010000,
		0b10110100, 0b10010110, 0b00010000, 0b10001010,

		0b01001101, 0b10010010, 0b10101110, 0b00001011,
		0b01001000, 0b11010011, 0b00101101, 0b10011100,
		0b01100111, 0b11011100, 0b10011010, 0b00110000,
		0b00101011, 0b11111100, 0b01110110, 0b01011110,
		0b11010110, 0b01111011, 0b11011110, 0b00111101,
		0b01010100, 0b10000001, 0b11001101, 0b11111010,
		0b01010011, 0b00010010, 0b01100111, 0b10101010,
		0b01100101, 0b01011010, 0b10100101, 0b10101011,
	}

	output := []byte{
		0b01000011, 0b00001000,
		0b10001010, 0b11101111,
		0b00111111, 0b00110001,
		0b11100101, 0b00001110,
		0b00110010, 0b11010000,
		0b11100011, 0b10101101,
		0b11010100, 0b00111010,
		0b01100010, 0b10110011,

		0b11101001, 0b11110100,
		0b00000100, 0b01111001,
		0b00010010, 0b00011000,
		0b01111001, 0b11111011,
		0b01011000, 0b00111001,
		0b10100011, 0b00110000,
		0b00111101, 0b11001011,
		0b01011010, 0b01001100,

		0b00001100, 0b10101101,
		0b11110000, 0b00011110,
		0b11101010, 0b11010011,
		0b11110011, 0b00111000,
		0b10010100, 0b11101100,
		0b00101110, 0b11011111,
		0b11101100, 0b10111010,
		0b01101100, 0b01000111,

		0b11111111, 0b01000111,
		0b10010010, 0b00010100,
		0b10010000, 0b00101111,
		0b11100010, 0b01111100,
		0b11010001, 0b11011111,
		0b10110100, 0b01011000,
		0b11111001, 0b10010111,
		0b01111100, 0b10001001,
	}

	transposeMatrix(input, 16, 4)
	if !bytes.Equal(input, output) {
		t.Error("output mismatch")
		t.FailNow()
	}
}
