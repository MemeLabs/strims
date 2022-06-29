// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package memory

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestMemoryQueue(t *testing.T) {
	m := NewQueue[any]()

	type foo struct {
		n int
	}

	for i := 0; i < 2; i++ {
		for j := 0; j < 4; j++ {
			m.Write(&foo{j})
		}

		for j := 0; j < 4; j++ {
			f, _ := m.Read()
			assert.Equal(t, j, f.(*foo).n)
		}
	}
}

func BenchmarkMemoryQueue(b *testing.B) {
	m := NewQueue[any]()

	type foo struct {
		n int
	}

	fs := make([]any, 0, 100)
	for i := 0; i < 100; i++ {
		fs = append(fs, &foo{i})
	}

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		for j := 0; j < 100; j++ {
			m.Write(fs[j%100])
		}

		for j := 0; j < 100; j++ {
			m.Read()
		}
	}
}
