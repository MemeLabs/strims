package main

import (
	"fmt"
	"strings"

	"github.com/MemeLabs/go-ppspp/hack/protogen/pkg/pgsutil"
	pgs "github.com/lyft/protoc-gen-star"
)

// TSRPCModule ...
type TSRPCModule struct {
	*pgs.ModuleBase
	c pgs.BuildContext
}

// TSRPC ...
func TSRPC() *TSRPCModule { return &TSRPCModule{ModuleBase: &pgs.ModuleBase{}} }

// InitContext ...
func (p *TSRPCModule) InitContext(c pgs.BuildContext) {
	p.ModuleBase.InitContext(c)
	p.c = c
}

// Name satisfies the generator.Plugin interface.
func (p *TSRPCModule) Name() string { return "tsrpc" }

// Execute ...
func (p *TSRPCModule) Execute(targets map[string]pgs.File, pkgs map[string]pgs.Package) []pgs.Artifact {
	for _, pkg := range pkgs {
		for _, f := range pkg.Files() {
			if len(f.Services()) != 0 {
				p.generate(f)
			}
		}
	}

	return p.Artifacts()
}

func (p *TSRPCModule) generate(f pgs.File) {
	path := strings.ReplaceAll(strings.TrimPrefix(f.FullyQualifiedName(), "."), ".", "/")
	name := fmt.Sprintf("%s/%s_rpc.ts", path, f.File().InputPath().BaseName())

	g := &generator{}
	g.generateFile(f)

	p.AddGeneratorFile(name, g.String())
}

type generator struct {
	pgsutil.Generator
}

func (g *generator) generateFile(f pgs.File) {
	g.generateImports(f)
	g.generateTypeRegistration(f)

	for _, s := range f.Services() {
		g.generateService(s)
	}
}

func (g *generator) generateImports(f pgs.File) {
	root := strings.Repeat("../", strings.Count(f.File().FullyQualifiedName(), "."))

	g.Linef(`import { RPCHost } from "%s../../rpc/host";`, root)
	g.Linef(`import { registerType } from "%s../../pb/registry";`, root)

EachService:
	for _, s := range f.Services() {
		for _, m := range s.Methods() {
			if m.ServerStreaming() {
				g.Linef(`import { Readable as GenericReadable } from "%s../../rpc/stream";`, root)
				break EachService
			}
		}
	}

	g.LineBreak()

	g.Line(`import {`)
	for _, s := range f.Services() {
		for _, m := range s.Methods() {
			g.Linef(`I%s,`, m.Input().Name())
			g.Linef(`%s,`, m.Input().Name())
			g.Linef(`%s,`, m.Output().Name())
		}
	}
	g.Linef(`} from "./%s";`, f.File().InputPath().BaseName())
	g.LineBreak()
}

func (g *generator) generateTypeRegistration(f pgs.File) {
	for _, s := range f.Services() {
		for _, m := range s.Methods() {
			g.Linef(`registerType("%s", %s);`, m.Input().FullyQualifiedName(), m.Input().Name())
			g.Linef(`registerType("%s", %s);`, m.Output().FullyQualifiedName(), m.Output().Name())
		}
	}
	g.LineBreak()
}

func (g *generator) generateService(s pgs.Service) {
	g.Linef(`export class %sClient {`, s.Name().UpperCamelCase())
	g.Line(`constructor(private readonly host: RPCHost) {}`)
	for _, m := range s.Methods() {
		input := m.Input().Name().String()
		output := m.Output().Name().UpperCamelCase().String()
		var outputMethod string
		if m.ServerStreaming() {
			outputMethod = "expectMany"
			output = fmt.Sprintf("GenericReadable<%s>", output)
		} else {
			outputMethod = "expectOne"
			output = fmt.Sprintf("Promise<%s>", output)
		}

		g.LineBreak()
		g.Linef(`public %s(arg: I%s = new %s()): %s {`, m.Name().LowerCamelCase(), input, input, output)
		g.Linef(`return this.host.%s(this.host.call("%s", new %s(arg)));`, outputMethod, m.FullyQualifiedName(), input)
		g.Line(`}`)
	}
	g.Line(`}`)
	g.LineBreak()
}
