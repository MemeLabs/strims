// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package options

import (
	"reflect"
)

func AssignDefaults[T any](options, defaults T) T {
	Assign(&defaults, options)
	return defaults
}

func AssignPtr[T any](dst *T, src *T) {
	if src != nil {
		assign(reflect.ValueOf(dst).Elem(), reflect.ValueOf(src).Elem())
	}
}

func Assign[T any](dst *T, src T) {
	assign(reflect.ValueOf(dst).Elem(), reflect.ValueOf(src))
}

func assign(dst, src reflect.Value) {
	for i := 0; i < dst.NumField(); i++ {
		if src.Field(i).Kind() == reflect.Struct {
			assign(dst.Field(i), src.Field(i))
		} else if !src.Field(i).IsZero() {
			dst.Field(i).Set(src.Field(i))
		}
	}
}
