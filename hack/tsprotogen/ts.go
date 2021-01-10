package main

import (
	"fmt"
	"log"
	"strings"

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

	g := &generator{c: p.c}
	g.generateFile(f)

	p.AddGeneratorFile(name, g.String())
}

type generator struct {
	indent int
	c      pgs.BuildContext
	f      pgs.File
	strings.Builder
}

func (g *generator) linef(v string, args ...interface{}) {
	g.line(fmt.Sprintf(v, args...))
}

func (g *generator) line(v string) {
	d := strings.Count(v, "{") - strings.Count(v, "}")
	if d < 0 {
		g.indent += d
	}
	g.WriteString(strings.Repeat("  ", g.indent))
	if d > 0 {
		g.indent += d
	}

	g.WriteString(v)
	g.lineBreak()
}

func (g *generator) lineBreak() {
	g.WriteRune('\n')
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

	g.linef(`import Reader from "%s../pb/reader";`, root)
	g.linef(`import Writer from "%s../pb/writer";`, root)
	g.lineBreak()

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
		g.line(`import {`)
		for _, t := range imports[i.InputPath().String()] {
			g.linef(
				"%s as %s,",
				strings.TrimPrefix(strings.TrimPrefix(t.FullyQualifiedName(), i.FullyQualifiedName()), "."),
				strings.ReplaceAll(strings.TrimPrefix(t.FullyQualifiedName(), "."), ".", "_"),
			)
			if e, ok := t.(pgs.Message); ok {
				g.linef(
					"I%[1]s as %s_I%[1]s",
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

		g.linef(`} from "%s%s/%s";`,
			prefix,
			strings.Join(iparts[commonPrefixLength:], "/"),
			i.File().InputPath().BaseName(),
		)
	}
	g.lineBreak()
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
	return fmt.Sprintf(`%s.%s%sOneOf`, o.Message().Name().UpperCamelCase(), prefix, o.Name().UpperCamelCase())
}

func (g *generator) generateMessage(m pgs.Message) {
	className := m.Name().UpperCamelCase()

	// constructor argument interface
	g.linef(`export interface I%s {`, className)
	for _, f := range m.NonOneOfFields() {
		undef := ""
		if f.Type().IsEmbed() {
			undef = " | undefined"
		}
		g.linef(`%s?: %s%s;`, g.fieldName(f), g.fieldInfo(f).tsIfType, undef)
	}
	for _, o := range m.OneOfs() {
		g.linef(`%s?: %s`, o.Name().LowerCamelCase(), g.oneOfNameWithPrefix(o, "I"))
	}
	g.line(`}`)
	g.lineBreak()

	g.linef(`export class %s {`, className)

	// properties
	for _, f := range m.NonOneOfFields() {
		fi := g.fieldInfo(f)
		if f.Type().IsEmbed() {
			g.linef(`%s: %s | undefined;`, g.fieldName(f), fi.tsType)
		} else {
			g.linef(`%s: %s = %s;`, g.fieldName(f), fi.tsType, fi.zeroValue)
		}
	}
	for _, o := range m.OneOfs() {
		g.linef(`%s: %s;`, o.Name().LowerCamelCase(), g.oneOfNameWithPrefix(o, "T"))
	}
	g.lineBreak()

	// constructor
	g.linef(`constructor(v?: I%s) {`, className)
	for _, f := range m.NonOneOfFields() {
		name := g.fieldName(f)
		fi := g.fieldInfo(f)
		if f.Type().IsRepeated() {
			if f.Type().Element().IsEmbed() {
				g.linef(`if (v?.%s) this.%s = v.%s.map(v => new %s(v));`, name, name, name, g.fieldInfo(f).tsBaseType)
			} else {
				g.linef(`if (v?.%s) this.%s = v.%s;`, name, name, name)
			}
		} else if f.Type().IsMap() {
			if f.Type().Element().IsEmbed() {
				g.linef(`if (v?.%s) this.%s = new Map((v.%s instanceof Map ? Array.from(v.%s.entries()) : Object.entries(v.%s)).map(([k, v]) => [k, new %s(v)]));`, name, name, name, name, name, g.fieldInfo(f).tsBaseType)
			} else {
				g.linef(`if (v?.%s) this.%s = v.%s instanceof Map ? v.%s : new Map(Object.entries(v.%s));`, name, name, name, name, name)
			}
		} else if f.Type().IsEmbed() {
			g.linef(`this.%s = v?.%s && new %s(v.%s);`, name, name, fi.tsType, name)
		} else {
			g.linef(`this.%s = v?.%s || %s;`, name, name, fi.zeroValue)
		}
	}
	for _, o := range m.OneOfs() {
		name := o.Name().LowerCamelCase()
		g.linef(`this.%s = new %s(v?.%s);`, name, g.oneOfName(o), name)
	}
	if len(m.Fields()) == 0 {
		g.line(`// noop`)
	}
	g.line(`}`)
	g.lineBreak()

	// encoder
	g.linef(`static encode(m: %s, w?: Writer): Writer {`, className)
	g.line(`if (!w) w = new Writer(1024);`)
	for _, f := range m.NonOneOfFields() {
		name := g.fieldName(f)
		fi := g.fieldInfo(f)
		wireKey := int(f.Descriptor().GetNumber() << 3)
		if f.Type().IsRepeated() {
			if f.Type().Element().IsEmbed() {
				g.linef(`for (const v of m.%s) %s.encode(v, w.uint32(%d).fork()).ldelim();`, name, fi.tsBaseType, wireKey|fi.wireType)
			} else if g.isPrimitiveNumeric(f.Type().Element().ProtoType()) {
				g.linef(`m.%s.reduce((w, v) => w.%s(v), w.uint32(%d).fork()).ldelim();`, name, fi.codecFunc, wireKey|2)
			} else {
				g.linef(`for (const v of m.%s) w.uint32(%d).%s(v);`, name, wireKey|fi.wireType, fi.codecFunc)
			}
		} else if f.Type().IsMap() {
			ki, _ := g.scalarFieldInfo(f.Type().Key().ProtoType())
			if f.Type().Element().IsEmbed() {
				g.linef(`for (const [k, v] of m.%s) %s.encode(v, w.uint32(%d).fork().uint32(%d).%s(k).uint32(%d).fork()).ldelim().ldelim();`, name, fi.tsBaseType, wireKey|wireTypeLDelim, 1<<3|ki.wireType, ki.codecFunc, 2<<3|fi.wireType)
			} else {
				g.linef(`for (const [k, v] of m.%s) w.uint32(%d).fork().uint32(%d).%s(k).uint32(%d).%s(v).ldelim();`, name, wireKey|wireTypeLDelim, 1<<3|ki.wireType, ki.codecFunc, 2<<3|fi.wireType, fi.codecFunc)
			}
		} else {
			if f.Type().IsEmbed() {
				g.linef(`if (m.%s) %s.encode(m.%s, w.uint32(%d).fork()).ldelim();`, name, fi.tsType, name, wireKey|fi.wireType)
			} else {
				g.linef(`if (m.%s) w.uint32(%d).%s(m.%s);`, name, wireKey|fi.wireType, fi.codecFunc, name)
			}
		}
	}
	for _, o := range m.OneOfs() {
		oname := o.Name().LowerCamelCase()
		g.linef(`switch (m.%s.case) {`, oname)
		for _, f := range o.Fields() {
			g.linef(`case %s.%s:`, className, g.oneOfCaseName(f))
			fname := g.fieldName(f)
			fi := g.fieldInfo(f)
			wireKey := int(f.Descriptor().GetNumber()<<3) | fi.wireType
			if f.Type().IsEmbed() {
				g.linef(`%s.encode(m.%s.%s, w.uint32(%d).fork()).ldelim();`, fi.tsType, oname, fname, wireKey)
			} else {
				g.linef(`w.uint32(%d).%s(m.%s.%s);`, wireKey, fi.codecFunc, oname, fname)
			}
			g.line(`break;`)
		}
		g.line(`}`)
	}
	g.line(`return w;`)
	g.line(`}`)
	g.lineBreak()

	// decoder
	g.linef(`static decode(r: Reader | Uint8Array, length?: number): %s {`, className)
	if len(m.Fields()) == 0 {
		g.line(`if (r instanceof Reader && length) r.skip(length);`)
		g.linef(`return new %s();`, className)
	} else {
		g.line(`r = r instanceof Reader ? r : new Reader(r);`)
		g.line(`const end = length === undefined ? r.len : r.pos + length;`)
		g.linef(`const m = new %s();`, className)
		g.line(`while (r.pos < end) {`)
		g.line(`const tag = r.uint32();`)
		g.line(`switch (tag >> 3) {`)
		for _, f := range m.Fields() {
			g.linef(`case %d:`, f.Descriptor().GetNumber())
			name := g.fieldName(f).String()
			fi := g.fieldInfo(f)
			if f.Type().IsRepeated() {
				if f.Type().Element().IsEmbed() {
					g.linef(`m.%s.push(%s.decode(r, r.uint32()));`, name, fi.tsBaseType)
				} else if g.isPrimitiveNumeric(f.Type().Element().ProtoType()) {
					g.linef(`for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.%s.push(r.%s());`, name, fi.codecFunc)
				} else {
					g.linef(`m.%s.push(r.%s())`, name, fi.codecFunc)
				}
			} else if f.Type().IsMap() {
				ki, _ := g.scalarFieldInfo(f.Type().Key().ProtoType())
				g.line(`{`)
				g.line(`const flen = r.uint32();`)
				g.line(`const fend = r.pos + flen;`)
				g.linef(`let key: %s;`, ki.tsBaseType)
				g.linef(`let value: %s;`, fi.tsBaseType)
				g.line(`while (r.pos < fend) {`)
				g.line(`const ftag = r.uint32();`)
				g.line(`switch (ftag >> 3) {`)
				g.line(`case 1:`)
				g.linef(`key = r.%s()`, ki.codecFunc)
				g.line(`break;`)
				g.line(`case 2:`)
				if f.Type().Element().IsEmbed() {
					g.linef(`value = %s.decode(r, r.uint32());`, fi.tsBaseType)
				} else {
					g.linef(`value = r.%s();`, fi.codecFunc)
				}
				g.line(`break;`)
				g.line(`}`)
				g.line(`}`)
				g.linef(`m.%s.set(key, value)`, name)
				g.line(`}`)
			} else if f.InOneOf() {
				oneOfName := f.OneOf().Name().LowerCamelCase()
				if f.Type().IsEmbed() {
					g.linef(`m.%s = new %s({ %s: %s.decode(r, r.uint32()) });`, oneOfName, g.oneOfName(f.OneOf()), name, fi.tsBaseType)
				} else {
					g.linef(`m.%s = new %s({ %s: r.%s() });`, oneOfName, g.oneOfName(f.OneOf()), name, fi.codecFunc)
				}
			} else {
				if f.Type().IsEmbed() {
					g.linef(`m.%s = %s.decode(r, r.uint32());`, name, fi.tsBaseType)
				} else {
					g.linef(`m.%s = r.%s();`, name, fi.codecFunc)
				}
			}
			g.line(`break;`)
		}
		g.line(`default:`)
		g.line(`r.skipType(tag & 7);`)
		g.line(`break;`)
		g.line(`}`)
		g.line(`}`)
		g.line(`return m;`)
	}
	g.line(`}`)
	g.line(`}`)
	g.lineBreak()

	if len(m.OneOfs()) != 0 || len(m.Messages()) != 0 || len(m.Enums()) != 0 {
		g.linef(`export namespace %s {`, className)

		for _, o := range m.OneOfs() {
			g.generateOneOf(o)
		}
		for _, m := range m.Messages() {
			g.generateMessage(m)
		}
		for _, e := range m.Enums() {
			g.generateEnum(e)
		}

		g.line(`}`)
		g.lineBreak()
	}
}

func (g *generator) oneOfCaseName(f pgs.Field) string {
	return fmt.Sprintf("%sCase.%s", f.OneOf().Name().UpperCamelCase(), f.Name().ScreamingSnakeCase())
}

func (g *generator) generateOneOf(o pgs.OneOf) {
	className := fmt.Sprintf("%sOneOf", o.Name().UpperCamelCase())
	caseName := fmt.Sprintf("%sCase", o.Name().UpperCamelCase())

	g.linef(`export enum %s {`, caseName)
	g.line(`NOT_SET = 0,`)
	for _, f := range o.Fields() {
		g.linef(`%s = %d,`, f.Name().ScreamingSnakeCase(), f.Descriptor().GetNumber())
	}
	g.line(`}`)
	g.lineBreak()

	// constructor argument interface
	g.linef(`export type I%s =`, className)
	g.linef(`{ case?: %s.NOT_SET }`, caseName)
	for _, f := range o.Fields() {
		g.linef(`|{ case?: %s, %s: %s }`, g.oneOfCaseName(f), g.fieldName(f), g.fieldInfo(f).tsIfType)
	}
	g.line(`;`)
	g.lineBreak()

	// cases
	g.linef(`export type T%s = Readonly<`, className)
	g.linef(`{ case: %s.NOT_SET }`, caseName)
	for _, f := range o.Fields() {
		g.linef(`|{ case: %s, %s: %s }`, g.oneOfCaseName(f), g.fieldName(f), g.fieldInfo(f).tsType)
	}
	g.line(`>;`)
	g.lineBreak()

	g.linef(`class %sImpl {`, className)

	// properties
	for _, f := range o.Fields() {
		fi := g.fieldInfo(f)
		if f.Type().IsEmbed() {
			g.linef(`%s: %s;`, g.fieldName(f), fi.tsType)
		} else {
			g.linef(`%s: %s;`, g.fieldName(f), fi.tsType)
		}
	}
	g.linef(`case: %s = %s.NOT_SET;`, caseName, caseName)
	g.lineBreak()

	// constructor
	g.linef(`constructor(v?: I%s) {`, className)
	for i, f := range o.Fields() {
		name := g.fieldName(f)
		if i != 0 {
			g.linef(`} else`)
		}
		g.linef(`if (v && "%s" in v) {`, name)
		g.linef(`this.case = %s;`, g.oneOfCaseName(f))
		if f.Type().IsEmbed() {
			g.linef(`this.%s = new %s(v.%s);`, name, g.fieldInfo(f).tsType, name)
		} else {
			g.linef(`this.%s = v.%s;`, name, name)
		}
	}
	g.line(`}`)
	g.line(`}`)
	g.line(`}`)
	g.lineBreak()

	g.linef(`export const %s = %sImpl as {`, className, className)
	g.linef(`new (): Readonly<{ case: %s.NOT_SET }>;`, caseName)
	g.linef(`new <T extends I%s>(v: T): Readonly<`, className)
	for _, f := range o.Fields() {
		fi := g.fieldInfo(f)
		g.linef(`T extends { %s: %s } ? { case: %s, %s: %s } :`, g.fieldName(f), fi.tsIfType, g.oneOfCaseName(f), g.fieldName(f), fi.tsType)
	}
	g.line(`never`)
	g.line(`>;`)
	g.line(`};`)
	g.lineBreak()
}

func (g *generator) generateEnum(e pgs.Enum) {
	g.linef(`export enum %s {`, e.Name().UpperCamelCase())
	for _, v := range e.Values() {
		g.linef(`%s = %d,`, v.Name().ScreamingSnakeCase(), v.Value())
	}
	g.line(`}`)
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

	// TODO: trim prefix from nested types
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
