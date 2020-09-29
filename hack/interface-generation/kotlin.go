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
{{range .Imports}}import gg.strims.ppspp.proto.{{.Filename | ToPascal}}.*
{{end}}
import java.util.concurrent.Future

class {{.Name}}Client(filepath: String) : RPCClient(filepath) {
{{range .Elements}}
    fun {{.Name | ToCamel}}(
        arg: {{.RequestType}} = {{.RequestType}}.newBuilder().build()
    ): {{if .StreamsReturns}}RPCResponseStream{{else}}Future{{end}}<{{.ReturnsType}}> =
        this.{{if .StreamsReturns}}callStreaming{{else}}callUnary{{end}}("{{.Name | ToCamel}}", arg)
{{end}}
}
`))
}

func (g *KotlinGen) Format(path string) error {
	log.Println("[WARN] formatting not implemented for kotlin")
	return nil
}
