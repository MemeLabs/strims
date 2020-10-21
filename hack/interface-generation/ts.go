package main

import (
	"log"
	"os"
	"os/exec"
	"path"
	"text/template"
)

type TsGen struct{}

func (g *TsGen) OutputPath(service ProtoService) string {
	return path.Join(wd, "src", "lib", "api", ToCamel(service.Name)+"Client.ts")
}

func (g *TsGen) Template() *template.Template {
	return template.Must(template.New("ts").Funcs(funcMap).Parse(`import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class {{.Name}} {
	constructor(private readonly host: RPCHost) {}

	{{range .Elements}}
	public {{.Name | ToCamel}}(arg: pb.I{{.RequestType}} = new pb.{{.RequestType}}()):  {{if .StreamsReturns}}GenericReadable{{else}}Promise{{end}}<pb.{{.ReturnsType}}> {
		return this.host.{{if .StreamsReturns}}expectMany{{else}}expectOne{{end}}(this.host.call("{{$.Name}}/{{.Name | ToPascal}}", new pb.{{.RequestType}}(arg)));
	}{{end}}
}
`))
}

func (g *TsGen) Format(path string) error {
	// check if prettier is installed
	cmd := exec.Command("npx", "prettier", "--version")
	err := cmd.Run()
	if err != nil {
		log.Println("[WARN] could not run prettier to format ts!", err)
	}

	formatCmd := exec.Command("npx", "prettier", "--write", path)
	formatCmd.Stdout = os.Stdout
	formatCmd.Stderr = os.Stderr

	return formatCmd.Run()
}
