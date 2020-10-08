package main

import (
	"log"
	"path"
	"text/template"
)

type KotlinGen struct{}

func (g *KotlinGen) OutputPath(service ProtoService) string {
	return path.Join(wd, "android", "app", "src", "main", "java", "gg", "strims", "ppspp", "rpc", service.Name+"Client.kt")
}

func (g *KotlinGen) Template() *template.Template {
	// TODO: don't hardcode imports
	return template.Must(template.New("ts").Funcs(funcMap).Parse(`package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class {{.Name}}Client(filepath: String) : RPCClient(filepath) {
{{range .Elements}}
    suspend fun {{.Name | ToCamel}}(
        arg: {{.RequestType}} = {{.RequestType}}()
    ): {{if .StreamsReturns}}RPCResponseStream<{{.ReturnsType}}>{{else}}{{.ReturnsType}}{{end}} =
        this.{{if .StreamsReturns}}callStreaming{{else}}callUnary{{end}}("{{$.Name}}/{{.Name | ToPascal}}", arg)
{{end}}
}
`))
}

func (g *KotlinGen) Format(path string) error {
	log.Println("[WARN] formatting not implemented for kotlin")
	return nil
}
