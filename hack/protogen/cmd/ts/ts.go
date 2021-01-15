package main

import (
	"fmt"
	"log"
	"strings"

	"github.com/MemeLabs/go-ppspp/hack/protogen/pkg/pgsutil"
	"github.com/golang/protobuf/protoc-gen-go/descriptor"
	pgs "github.com/lyft/protoc-gen-star"
)

// TSModule ...
type TSModule struct {
	*pgs.ModuleBase
	c pgs.BuildContext
}

// TS ...
func TS() *TSModule { return &TSModule{ModuleBase: &pgs.ModuleBase{}} }

// InitContext ...
func (p *TSModule) InitContext(c pgs.BuildContext) {
	p.ModuleBase.InitContext(c)
	p.c = c
}

// Name satisfies the generator.Plugin interface.
func (p *TSModule) Name() string { return "ts" }

// Execute ...
func (p *TSModule) Execute(targets map[string]pgs.File, pkgs map[string]pgs.Package) []pgs.Artifact {
	for _, pkg := range pkgs {
		for _, f := range pkg.Files() {
			p.generate(f)
		}
	}

	return p.Artifacts()
}

func (p *TSModule) generate(f pgs.File) {
	path := strings.ReplaceAll(strings.TrimPrefix(f.FullyQualifiedName(), "."), ".", "/")
	name := fmt.Sprintf("%s/%s.ts", path, f.File().InputPath().BaseName())

	g := &generator{}
	g.generateFile(f)

	p.AddGeneratorFile(name, g.String())
}

type generator struct {
	pgsutil.Generator
}

func (g *generator) generateFile(f pgs.File) {
	g.generateImports(f)

	for _, m := range f.Messages() {
		g.generateMessage(m)
	}
	for _, e := range f.Enums() {
		g.generateEnum(e)
	}
}

func (g *generator) generateImports(f pgs.File) {
	root := strings.Repeat("../", strings.Count(f.File().FullyQualifiedName(), "."))

	g.Linef(`import Reader from "%s../lib/pb/reader";`, root)
	g.Linef(`import Writer from "%s../lib/pb/writer";`, root)
	g.LineBreak()

	imports := map[string]map[string]pgs.Entity{}
	for _, m := range f.AllMessages() {
	EachField:
		for _, f := range m.Fields() {
			var e pgs.Entity
			switch {
			case f.Type().IsEmbed():
				e = f.Type().Embed()
			case f.Type().IsEnum():
				e = f.Type().Enum()
			default:
				continue EachField
			}

			fk := e.File().InputPath().String()
			ek := e.FullyQualifiedName()
			if _, ok := imports[fk]; !ok {
				imports[fk] = map[string]pgs.Entity{}
			}
			imports[fk][ek] = e
		}
	}
	for _, i := range f.Imports() {
		g.Line(`import {`)
		for _, t := range imports[i.InputPath().String()] {
			g.Linef(
				"%s as %s,",
				strings.TrimPrefix(strings.TrimPrefix(t.FullyQualifiedName(), i.FullyQualifiedName()), "."),
				strings.ReplaceAll(strings.TrimPrefix(t.FullyQualifiedName(), "."), ".", "_"),
			)
			if e, ok := t.(pgs.Message); ok {
				g.Linef(
					"I%[1]s as %s_I%[1]s,",
					e.Name().UpperCamelCase().String(),
					strings.ReplaceAll(strings.TrimPrefix(e.File().FullyQualifiedName(), "."), ".", "_"),
				)
			}
		}

		iparts := strings.Split(strings.TrimPrefix(i.FullyQualifiedName(), "."), ".")
		fparts := strings.Split(strings.TrimPrefix(f.FullyQualifiedName(), "."), ".")

		l := len(iparts)
		if fl := len(fparts); fl < l {
			l = fl
		}

		commonPrefixLength := 0
		for i := 0; i < l; i++ {
			if iparts[i] == fparts[i] {
				commonPrefixLength++
			} else {
				break
			}
		}
		prefix := "."
		if uniquePrefixLength := len(fparts) - commonPrefixLength; uniquePrefixLength != 0 {
			prefix = strings.Repeat("../", uniquePrefixLength)
		}

		g.Linef(`} from "%s%s/%s";`,
			prefix,
			strings.Join(iparts[commonPrefixLength:], "/"),
			i.File().InputPath().BaseName(),
		)
	}
	g.LineBreak()
}

func (g *generator) fieldName(f pgs.Field) pgs.Name {
	return f.Name().LowerCamelCase()
}

func (g *generator) fieldType(f pgs.Field) string {
	return g.fieldInfo(f).tsType
}

func (g *generator) oneOfName(o pgs.OneOf) string {
	return g.oneOfNameWithPrefix(o, "")
}

func (g *generator) oneOfNameWithPrefix(o pgs.OneOf, prefix string) string {
	return fmt.Sprintf(`%s.%s%s`, o.Message().Name().UpperCamelCase(), prefix, o.Name().UpperCamelCase())
}

func (g *generator) generateMessage(m pgs.Message) {
	className := m.Name().UpperCamelCase()

	// constructor argument interface
	g.Linef(`export interface I%s {`, className)
	for _, f := range m.NonOneOfFields() {
		undef := ""
		if f.Type().IsEmbed() {
			undef = " | undefined"
		}
		g.Linef(`%s?: %s%s;`, g.fieldName(f), g.fieldInfo(f).tsIfType, undef)
	}
	for _, o := range m.OneOfs() {
		g.Linef(`%s?: %s`, o.Name().LowerCamelCase(), g.oneOfNameWithPrefix(o, "I"))
	}
	g.Line(`}`)
	g.LineBreak()

	g.Linef(`export class %s {`, className)

	// properties
	for _, f := range m.NonOneOfFields() {
		fi := g.fieldInfo(f)
		if f.Type().IsEmbed() {
			g.Linef(`%s: %s | undefined;`, g.fieldName(f), fi.tsType)
		} else {
			g.Linef(`%s: %s = %s;`, g.fieldName(f), fi.tsType, fi.zeroValue)
		}
	}
	for _, o := range m.OneOfs() {
		g.Linef(`%s: %s;`, o.Name().LowerCamelCase(), g.oneOfNameWithPrefix(o, "T"))
	}
	g.LineBreak()

	// constructor
	g.Linef(`constructor(v?: I%s) {`, className)
	for _, f := range m.NonOneOfFields() {
		name := g.fieldName(f)
		fi := g.fieldInfo(f)
		if f.Type().IsRepeated() {
			if f.Type().Element().IsEmbed() {
				g.Linef(`if (v?.%s) this.%s = v.%s.map(v => new %s(v));`, name, name, name, g.fieldInfo(f).tsBaseType)
			} else {
				g.Linef(`if (v?.%s) this.%s = v.%s;`, name, name, name)
			}
		} else if f.Type().IsMap() {
			if f.Type().Element().IsEmbed() {
				g.Linef(`if (v?.%s) this.%s = new Map((v.%s instanceof Map ? Array.from(v.%s.entries()) : Object.entries(v.%s)).map(([k, v]) => [k, new %s(v)]));`, name, name, name, name, name, g.fieldInfo(f).tsBaseType)
			} else {
				g.Linef(`if (v?.%s) this.%s = v.%s instanceof Map ? v.%s : new Map(Object.entries(v.%s));`, name, name, name, name, name)
			}
		} else if f.Type().IsEmbed() {
			g.Linef(`this.%s = v?.%s && new %s(v.%s);`, name, name, fi.tsType, name)
		} else {
			g.Linef(`this.%s = v?.%s || %s;`, name, name, fi.zeroValue)
		}
	}
	for _, o := range m.OneOfs() {
		name := o.Name().LowerCamelCase()
		g.Linef(`this.%s = new %s(v?.%s);`, name, g.oneOfName(o), name)
	}
	if len(m.Fields()) == 0 {
		g.Line(`// noop`)
	}
	g.Line(`}`)
	g.LineBreak()

	// encoder
	g.Linef(`static encode(m: %s, w?: Writer): Writer {`, className)
	g.Line(`if (!w) w = new Writer();`)
	for _, f := range m.NonOneOfFields() {
		name := g.fieldName(f)
		fi := g.fieldInfo(f)
		wireKey := int(f.Descriptor().GetNumber() << 3)
		if f.Type().IsRepeated() {
			if f.Type().Element().IsEmbed() {
				g.Linef(`for (const v of m.%s) %s.encode(v, w.uint32(%d).fork()).ldelim();`, name, fi.tsBaseType, wireKey|fi.wireType)
			} else if g.isPrimitiveNumeric(f.Type().Element().ProtoType()) {
				g.Linef(`m.%s.reduce((w, v) => w.%s(v), w.uint32(%d).fork()).ldelim();`, name, fi.codecFunc, wireKey|2)
			} else {
				g.Linef(`for (const v of m.%s) w.uint32(%d).%s(v);`, name, wireKey|fi.wireType, fi.codecFunc)
			}
		} else if f.Type().IsMap() {
			ki, _ := g.scalarFieldInfo(f.Type().Key().ProtoType())
			if f.Type().Element().IsEmbed() {
				g.Linef(`for (const [k, v] of m.%s) %s.encode(v, w.uint32(%d).fork().uint32(%d).%s(k).uint32(%d).fork()).ldelim().ldelim();`, name, fi.tsBaseType, wireKey|wireTypeLDelim, 1<<3|ki.wireType, ki.codecFunc, 2<<3|fi.wireType)
			} else {
				g.Linef(`for (const [k, v] of m.%s) w.uint32(%d).fork().uint32(%d).%s(k).uint32(%d).%s(v).ldelim();`, name, wireKey|wireTypeLDelim, 1<<3|ki.wireType, ki.codecFunc, 2<<3|fi.wireType, fi.codecFunc)
			}
		} else {
			if f.Type().IsEmbed() {
				g.Linef(`if (m.%s) %s.encode(m.%s, w.uint32(%d).fork()).ldelim();`, name, fi.tsType, name, wireKey|fi.wireType)
			} else {
				g.Linef(`if (m.%s) w.uint32(%d).%s(m.%s);`, name, wireKey|fi.wireType, fi.codecFunc, name)
			}
		}
	}
	for _, o := range m.OneOfs() {
		oname := o.Name().LowerCamelCase()
		g.Linef(`switch (m.%s.case) {`, oname)
		for _, f := range o.Fields() {
			g.Linef(`case %s.%s:`, className, g.oneOfCaseName(f))
			fname := g.fieldName(f)
			fi := g.fieldInfo(f)
			wireKey := int(f.Descriptor().GetNumber()<<3) | fi.wireType
			if f.Type().IsEmbed() {
				g.Linef(`%s.encode(m.%s.%s, w.uint32(%d).fork()).ldelim();`, fi.tsType, oname, fname, wireKey)
			} else {
				g.Linef(`w.uint32(%d).%s(m.%s.%s);`, wireKey, fi.codecFunc, oname, fname)
			}
			g.Line(`break;`)
		}
		g.Line(`}`)
	}
	g.Line(`return w;`)
	g.Line(`}`)
	g.LineBreak()

	// decoder
	g.Linef(`static decode(r: Reader | Uint8Array, length?: number): %s {`, className)
	if len(m.Fields()) == 0 {
		g.Line(`if (r instanceof Reader && length) r.skip(length);`)
		g.Linef(`return new %s();`, className)
	} else {
		g.Line(`r = r instanceof Reader ? r : new Reader(r);`)
		g.Line(`const end = length === undefined ? r.len : r.pos + length;`)
		g.Linef(`const m = new %s();`, className)
		g.Line(`while (r.pos < end) {`)
		g.Line(`const tag = r.uint32();`)
		g.Line(`switch (tag >> 3) {`)
		for _, f := range m.Fields() {
			g.Linef(`case %d:`, f.Descriptor().GetNumber())
			name := g.fieldName(f).String()
			fi := g.fieldInfo(f)
			if f.Type().IsRepeated() {
				if f.Type().Element().IsEmbed() {
					g.Linef(`m.%s.push(%s.decode(r, r.uint32()));`, name, fi.tsBaseType)
				} else if g.isPrimitiveNumeric(f.Type().Element().ProtoType()) {
					g.Linef(`for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.%s.push(r.%s());`, name, fi.codecFunc)
				} else {
					g.Linef(`m.%s.push(r.%s())`, name, fi.codecFunc)
				}
			} else if f.Type().IsMap() {
				ki, _ := g.scalarFieldInfo(f.Type().Key().ProtoType())
				g.Line(`{`)
				g.Line(`const flen = r.uint32();`)
				g.Line(`const fend = r.pos + flen;`)
				g.Linef(`let key: %s;`, ki.tsBaseType)
				g.Linef(`let value: %s;`, fi.tsBaseType)
				g.Line(`while (r.pos < fend) {`)
				g.Line(`const ftag = r.uint32();`)
				g.Line(`switch (ftag >> 3) {`)
				g.Line(`case 1:`)
				g.Linef(`key = r.%s()`, ki.codecFunc)
				g.Line(`break;`)
				g.Line(`case 2:`)
				if f.Type().Element().IsEmbed() {
					g.Linef(`value = %s.decode(r, r.uint32());`, fi.tsBaseType)
				} else {
					g.Linef(`value = r.%s();`, fi.codecFunc)
				}
				g.Line(`break;`)
				g.Line(`}`)
				g.Line(`}`)
				g.Linef(`m.%s.set(key, value)`, name)
				g.Line(`}`)
			} else if f.InOneOf() {
				oneOfName := f.OneOf().Name().LowerCamelCase()
				if f.Type().IsEmbed() {
					g.Linef(`m.%s = new %s({ %s: %s.decode(r, r.uint32()) });`, oneOfName, g.oneOfName(f.OneOf()), name, fi.tsBaseType)
				} else {
					g.Linef(`m.%s = new %s({ %s: r.%s() });`, oneOfName, g.oneOfName(f.OneOf()), name, fi.codecFunc)
				}
			} else {
				if f.Type().IsEmbed() {
					g.Linef(`m.%s = %s.decode(r, r.uint32());`, name, fi.tsBaseType)
				} else {
					g.Linef(`m.%s = r.%s();`, name, fi.codecFunc)
				}
			}
			g.Line(`break;`)
		}
		g.Line(`default:`)
		g.Line(`r.skipType(tag & 7);`)
		g.Line(`break;`)
		g.Line(`}`)
		g.Line(`}`)
		g.Line(`return m;`)
	}
	g.Line(`}`)
	g.Line(`}`)
	g.LineBreak()

	if len(m.OneOfs()) != 0 || len(m.Messages()) != 0 || len(m.Enums()) != 0 {
		g.Linef(`export namespace %s {`, className)

		for _, o := range m.OneOfs() {
			g.generateOneOf(o)
		}
		for _, m := range m.Messages() {
			g.generateMessage(m)
		}
		for _, e := range m.Enums() {
			g.generateEnum(e)
		}

		g.Line(`}`)
		g.LineBreak()
	}
}

func (g *generator) oneOfCaseName(f pgs.Field) string {
	return fmt.Sprintf("%sCase.%s", f.OneOf().Name().UpperCamelCase(), f.Name().ScreamingSnakeCase())
}

func (g *generator) generateOneOf(o pgs.OneOf) {
	className := fmt.Sprintf("%s", o.Name().UpperCamelCase())
	caseName := fmt.Sprintf("%sCase", o.Name().UpperCamelCase())

	g.Linef(`export enum %s {`, caseName)
	g.Line(`NOT_SET = 0,`)
	for _, f := range o.Fields() {
		g.Linef(`%s = %d,`, f.Name().ScreamingSnakeCase(), f.Descriptor().GetNumber())
	}
	g.Line(`}`)
	g.LineBreak()

	// constructor argument interface
	g.Linef(`export type I%s =`, className)
	g.Linef(`{ case?: %s.NOT_SET }`, caseName)
	for _, f := range o.Fields() {
		g.Linef(`|{ case?: %s, %s: %s }`, g.oneOfCaseName(f), g.fieldName(f), g.fieldInfo(f).tsIfType)
	}
	g.Line(`;`)
	g.LineBreak()

	// cases
	g.Linef(`export type T%s = Readonly<`, className)
	g.Linef(`{ case: %s.NOT_SET }`, caseName)
	for _, f := range o.Fields() {
		g.Linef(`|{ case: %s, %s: %s }`, g.oneOfCaseName(f), g.fieldName(f), g.fieldInfo(f).tsType)
	}
	g.Line(`>;`)
	g.LineBreak()

	g.Linef(`class %sImpl {`, className)

	// properties
	for _, f := range o.Fields() {
		fi := g.fieldInfo(f)
		if f.Type().IsEmbed() {
			g.Linef(`%s: %s;`, g.fieldName(f), fi.tsType)
		} else {
			g.Linef(`%s: %s;`, g.fieldName(f), fi.tsType)
		}
	}
	g.Linef(`case: %s = %s.NOT_SET;`, caseName, caseName)
	g.LineBreak()

	// constructor
	g.Linef(`constructor(v?: I%s) {`, className)
	for i, f := range o.Fields() {
		name := g.fieldName(f)
		if i != 0 {
			g.Linef(`} else`)
		}
		g.Linef(`if (v && "%s" in v) {`, name)
		g.Linef(`this.case = %s;`, g.oneOfCaseName(f))
		if f.Type().IsEmbed() {
			g.Linef(`this.%s = new %s(v.%s);`, name, g.fieldInfo(f).tsType, name)
		} else {
			g.Linef(`this.%s = v.%s;`, name, name)
		}
	}
	g.Line(`}`)
	g.Line(`}`)
	g.Line(`}`)
	g.LineBreak()

	g.Linef(`export const %s = %sImpl as {`, className, className)
	g.Linef(`new (): Readonly<{ case: %s.NOT_SET }>;`, caseName)
	g.Linef(`new <T extends I%s>(v: T): Readonly<`, className)
	for _, f := range o.Fields() {
		fi := g.fieldInfo(f)
		g.Linef(`T extends { %s: %s } ? { case: %s, %s: %s } :`, g.fieldName(f), fi.tsIfType, g.oneOfCaseName(f), g.fieldName(f), fi.tsType)
	}
	g.Line(`never`)
	g.Line(`>;`)
	g.Line(`};`)
	g.LineBreak()
}

func (g *generator) generateEnum(e pgs.Enum) {
	g.Linef(`export enum %s {`, e.Name().UpperCamelCase())
	for _, v := range e.Values() {
		g.Linef(`%s = %d,`, v.Name().ScreamingSnakeCase(), v.Value())
	}
	g.Line(`}`)
}

type fieldInfo struct {
	tsType     string
	tsIfType   string
	tsBaseType string
	codecFunc  string
	zeroValue  string
	wireType   int
}

const (
	wireTypeVarint = 0
	wireType64Bit  = 1
	wireTypeLDelim = 2
	wireType32Bit  = 5
)

// get the ts type name for the entity relative to m
func (g *generator) entityName(f pgs.FieldType, m pgs.Message) (tsType, tsIfType, codecFunc string) {
	ifPrefix := "I"

	var e pgs.Entity
	switch {
	case f.IsEmbed():
		e = f.Embed()
	case f.IsRepeated():
		e = f.Element().Embed()
	case f.IsMap():
		el := f.Element()
		if i, ok := g.scalarFieldInfo(el.ProtoType()); ok {
			return i.tsType, i.tsIfType, i.codecFunc
		}
		e = el.Embed()
	case f.IsEnum():
		e = f.Enum()
		ifPrefix = ""
	}

	if e.File() != m.File() {
		prefix := strings.ReplaceAll(strings.TrimPrefix(e.File().FullyQualifiedName(), "."), ".", "_")
		tsType = fmt.Sprintf("%s_%s", prefix, e.Name().UpperCamelCase().String())
		tsIfType = fmt.Sprintf("%s_%s%s", prefix, ifPrefix, e.Name().UpperCamelCase().String())
		return
	}

	tsType = strings.TrimPrefix(strings.TrimPrefix(e.FullyQualifiedName(), e.File().FullyQualifiedName()), ".")

	i := strings.LastIndex(tsType, ".") + 1
	tsIfType = fmt.Sprintf("%s%s%s", tsType[:i], ifPrefix, tsType[i:])

	return
}

func (g *generator) isPrimitiveNumeric(p pgs.ProtoType) bool {
	switch p {
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_STRING):
		fallthrough
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_MESSAGE):
		fallthrough
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_BYTES):
		return false
	default:
		return true
	}
}

func (g *generator) scalarFieldInfo(p pgs.ProtoType) (fieldInfo, bool) {
	switch p {
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_DOUBLE):
		return fieldInfo{"number", "number", "number", "double", "0", wireType64Bit}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_FLOAT):
		return fieldInfo{"number", "number", "number", "float", "0", wireType32Bit}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_INT64):
		return fieldInfo{"bigint", "bigint", "BigInt", "int64", "BigInt(0)", wireTypeVarint}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_UINT64):
		return fieldInfo{"bigint", "bigint", "BigInt", "uint64", "BigInt(0)", wireTypeVarint}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_INT32):
		return fieldInfo{"number", "number", "number", "int32", "0", wireTypeVarint}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_UINT32):
		return fieldInfo{"number", "number", "number", "uint32", "0", wireTypeVarint}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_FIXED64):
		return fieldInfo{"bigint", "bigint", "BigInt", "sfixed64", "BigInt(0)", wireType64Bit}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_FIXED32):
		return fieldInfo{"number", "number", "number", "fixed32", "0", wireType32Bit}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_BOOL):
		return fieldInfo{"boolean", "boolean", "boolean", "bool", "false", wireTypeVarint}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_STRING):
		return fieldInfo{"string", "string", "string", "string", `""`, wireTypeLDelim}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_BYTES):
		return fieldInfo{"Uint8Array", "Uint8Array", "Uint8Array", "bytes", "new Uint8Array()", wireTypeLDelim}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_SFIXED32):
		return fieldInfo{"number", "number", "number", "sfixed32", "0", wireType32Bit}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_SFIXED64):
		return fieldInfo{"bigint", "bigint", "BigInt", "sfixed64", "BigInt(0)", wireType64Bit}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_SINT32):
		return fieldInfo{"number", "number", "number", "sint32", "0", wireTypeVarint}, true
	case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_SINT64):
		return fieldInfo{"bigint", "bigint", "BigInt", "sint64", "BigInt(0)", wireTypeVarint}, true
	default:
		return fieldInfo{}, false
	}
}

func (g *generator) fieldInfo(f pgs.Field) (t fieldInfo) {
	t, ok := g.scalarFieldInfo(f.Type().ProtoType())
	if !ok {
		switch f.Type().ProtoType() {
		case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_MESSAGE):
			tsType, tsIfType, codecFunc := g.entityName(f.Type(), f.Message())
			t = fieldInfo{tsType, tsIfType, tsType, codecFunc, "new " + tsType + "()", wireTypeLDelim}
		case pgs.ProtoType(descriptor.FieldDescriptorProto_TYPE_ENUM):
			tsType, tsIfType, _ := g.entityName(f.Type(), f.Message())
			t = fieldInfo{tsType, tsIfType, tsType, "uint32", "0", wireTypeVarint}
		default:
			log.Panicln("unknown type for", f.Name())
		}
	}

	if f.Type().IsMap() {
		kt, _ := g.scalarFieldInfo(f.Type().Key().ProtoType())
		t.tsType = fmt.Sprintf(`Map<%s, %s>`, kt.tsType, t.tsType)
		t.tsIfType = fmt.Sprintf(`Map<%s, %s> | { [key: %s]: %s }`, kt.tsType, t.tsIfType, kt.tsType, t.tsIfType)
		t.zeroValue = "new Map()"
	} else if f.Type().IsRepeated() {
		t.tsType = t.tsType + "[]"
		t.tsIfType = t.tsIfType + "[]"
		t.zeroValue = "[]"
	}
	return
}
