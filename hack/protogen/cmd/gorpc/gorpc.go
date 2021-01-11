package main

import (
	"strings"
	"text/template"

	pgs "github.com/lyft/protoc-gen-star"
	pgsgo "github.com/lyft/protoc-gen-star/lang/go"
)

// GoRPCModule ...
type GoRPCModule struct {
	*pgs.ModuleBase
	ctx pgsgo.Context
	tpl *template.Template
}

// GoRPC ...
func GoRPC() *GoRPCModule { return &GoRPCModule{ModuleBase: &pgs.ModuleBase{}} }

// InitContext ...
func (p *GoRPCModule) InitContext(c pgs.BuildContext) {
	p.ModuleBase.InitContext(c)
	p.ctx = pgsgo.InitContext(c.Parameters())

	tpl := template.New("gorpc").Funcs(map[string]interface{}{
		"package": p.ctx.PackageName,
		"name":    p.ctx.Name,
	})

	p.tpl = template.Must(tpl.Parse(rpcTpl))
}

// Name satisfies the generator.Plugin interface.
func (p *GoRPCModule) Name() string { return "gorpc" }

// Execute ...
func (p *GoRPCModule) Execute(targets map[string]pgs.File, pkgs map[string]pgs.Package) []pgs.Artifact {
	for _, t := range targets {
		p.generate(t)
	}

	return p.Artifacts()
}

func (p *GoRPCModule) generate(f pgs.File) {
	if len(f.Services()) == 0 {
		return
	}

	name := p.ctx.OutputPath(f).SetExt(".rpc.go").String()
	name = strings.TrimPrefix(strings.TrimPrefix(name, p.ctx.Params().Str("module")), "/")
	p.AddGeneratorTemplateFile(name, p.tpl, f)
}

const rpcTpl = `package {{ package . }}

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
)

{{range .Services}}
{{$svcName := .Name.UpperCamelCase}}
// Register{{$svcName}}Service ...
func Register{{$svcName}}Service(host api.ServiceRegistry, service {{$svcName}}Service) {
{{range .Methods}}  host.RegisterMethod("{{.FullyQualifiedName}}", service.{{.Name.UpperCamelCase}})
{{end}}}

// {{$svcName}}Service ...
type {{$svcName}}Service interface {
{{range .Methods}}  {{.Name.UpperCamelCase}} (
	  ctx context.Context,
	  req *{{.Input.Name.UpperCamelCase}},
  ) ({{if .ServerStreaming}}<-chan {{end}}*{{.Output.Name.UpperCamelCase}}, error)
{{end}}}

// {{$svcName}}Client ...
type {{$svcName}}Client struct {
	client api.Caller
}

// New{{$svcName}}Client ...
func New{{$svcName}}Client(client api.Caller) *{{$svcName}}Client {
	return &{{$svcName}}Client{client}
}
{{range .Methods}}
// {{.Name.UpperCamelCase}} ...
func (c *{{$svcName}}Client) {{.Name.UpperCamelCase}} (
	ctx context.Context,
	req *{{.Input.Name.UpperCamelCase}},
	res {{if .ServerStreaming}}chan {{end}}*{{.Output.Name.UpperCamelCase}},
) error {
	return c.client.{{if .ServerStreaming}}CallStreaming{{else}}CallUnary{{end}}(ctx, "{{.FullyQualifiedName}}", req, res)
}
{{end}}
{{end}}
`
