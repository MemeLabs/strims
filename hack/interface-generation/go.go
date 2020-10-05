package main

import (
	"fmt"
	"go/format"
	"io/ioutil"
	"os"
	"path"
	"strings"
	"text/template"

	"golang.org/x/tools/imports"
)

type GoClientGen struct{}

func (g *GoClientGen) OutputPath(service ProtoService) string {
	return path.Join(wd, "pkg", "api", strings.ToLower(service.Name)+".service.go")
}

func (g *GoClientGen) Template() *template.Template {
	return template.Must(template.New("ts").Funcs(funcMap).Parse(`package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

func Register{{.Name}}Service(host *rpc.Host, service {{.Name}}Service) {
	host.RegisterService("{{.Name}}", service);
}

type {{.Name}}Service interface {
{{range .Elements}}  {{.Name | ToPascal}} (
	  ctx context.Context,
	  req *pb.{{.RequestType}},
  ) ({{if .StreamsReturns}}<-chan {{end}}*pb.{{.ReturnsType}}, error)
{{end}}}
`))
}

func (g *GoClientGen) Format(path string) error {
	return gofmt(path)
}

type GoServiceGen struct{}

func (g *GoServiceGen) OutputPath(service ProtoService) string {
	return path.Join(wd, "pkg", "api", strings.ToLower(service.Name)+".client.go")
}

func (g *GoServiceGen) Template() *template.Template {
	return template.Must(template.New("ts").Funcs(funcMap).Parse(`package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

type {{.Name}}Client struct {
	client  *rpc.Client
}

// New ...
func New{{.Name}}Client(client *rpc.Client) *{{.Name}}Client {
	return &{{.Name}}Client{client}
}
{{range .Elements}}
// {{.Name | ToPascal}} ...
func (c *{{$.Name}}Client) {{.Name | ToPascal}} (
	ctx context.Context,
	req *pb.{{.RequestType}},
	res {{if .StreamsReturns}}chan {{end}}*pb.{{.ReturnsType}},
) error {
	return c.client.{{if .StreamsReturns}}CallStreaming{{else}}CallUnary{{end}}(ctx, "{{$.Name}}/{{.Name | ToPascal}}", req, res)
}
{{end}}
`))
}

func (g *GoServiceGen) Format(path string) error {
	return gofmt(path)
}

func gofmt(path string) error {
	preFormat, err := ioutil.ReadFile(path)
	if err != nil {
		return fmt.Errorf("failed to read pre-formatted file: %v", err)
	}

	formattedFile, err := format.Source(preFormat)
	if err != nil {
		return fmt.Errorf("failed to format generated file: %v", err)
	}

	imp, err := imports.Process(path, formattedFile, nil)
	if err != nil {
		return fmt.Errorf("failed to import pkgs: %v", err)
	}

	if err = ioutil.WriteFile(path, imp, os.ModePerm); err != nil {
		return fmt.Errorf("failed to write formatted file: %v", err)
	}

	return nil
}
