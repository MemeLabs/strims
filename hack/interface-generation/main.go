package main

import (
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"strings"
	"text/template"

	"github.com/emicklei/proto"
)

var (
	generators = []Generator{&TsGen{}, &SwiftGen{}, &KotlinGen{}}
	funcMap    = make(template.FuncMap)
	wd         string
)

func ToCamel(input string) string {
	return strings.ToLower(string(input[0])) + input[1:]
}

// Generator can be implemented for any language to generate client definitions
type Generator interface {
	OutputPath(*proto.Service) string
	Template() *template.Template
	Format(string) error
}

func main() {
	wd, err := os.Getwd()
	if err != nil {
		log.Fatal(err)
	}
	funcMap["ToCamel"] = ToCamel

	err = filepath.Walk(path.Join(wd, "schema"), func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.IsDir() {
			return nil
		}
		f, err := os.Open(path)
		if err != nil {
			return err
		}
		parser := proto.NewParser(f)
		res, err := parser.Parse()
		if err != nil {
			return err
		}
		proto.Walk(res, proto.WithService(genService))

		return nil
	})

	if err != nil {
		log.Fatal(err)
	}
}

func genService(service *proto.Service) {
	for _, generator := range generators {
		file, err := os.OpenFile(generator.OutputPath(service), os.O_CREATE|os.O_WRONLY, os.ModePerm)
		if err != nil {
			log.Fatalf("Could not open output file %s: %v", generator.OutputPath(service), err)
		}
		if err := file.Truncate(0); err != nil {
			log.Fatalf("Could not truncate output file %s: %v", file.Name(), err)
		}
		template := generator.Template()
		if err := writeTemplate(template, service, file); err != nil {
			log.Fatalf("Could not write template to file %s: %v", file.Name(), err)
		}
		if err := generator.Format(generator.OutputPath(service)); err != nil {
			log.Fatalf("Could not format file %s: %v", file.Name(), err)
		}
	}
}

func writeTemplate(t *template.Template, service *proto.Service, w io.WriteCloser) error {
	if err := t.Execute(w, service); err != nil {
		return fmt.Errorf("failed to execute template: %w", err)
	}
	if err := w.Close(); err != nil {
		return fmt.Errorf("failed to close writer: %w", err)
	}
	return nil
}

type TsGen struct{}

func (g *TsGen) OutputPath(service *proto.Service) string {
	return path.Join(wd, "src", "lib", "api", ToCamel(service.Name)+"Client.ts")
}

func (g *TsGen) Template() *template.Template {
	return template.Must(template.New("ts").Funcs(funcMap).Parse(`import * as pb from "../pb";
import { RPCHost } from "./rpc_host";
import { Readable as GenericReadable } from "./stream";

export default class {{.Name}} extends RPCHost {
	{{range .Elements}}
	public {{.Name}}(v: pb.I{{.RequestType}} = new pb.{{.RequestType}}()):  {{if .StreamsReturns}}GenericReadable{{else}}Promise{{end}}<pb.{{.ReturnsType}}> {
		return this.{{if .StreamsReturns}}expectMany{{else}}expectOne{{end}}(this.call("{{.Name | ToCamel}}", new pb.{{.RequestType}}(v)));
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

type SwiftGen struct{}

func (g *SwiftGen) OutputPath(service *proto.Service) string {
	return path.Join(wd, "ios", "App", "App", service.Name+"Client.swift")
}

func (g *SwiftGen) Template() *template.Template {
	return template.Must(template.New("ts").Funcs(funcMap).Parse(`// swift-format-ignore-file
//
//  {{.Name}}Client.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class {{.Name}}Client: RPCClient {
  {{range .Elements}}public func {{.Name | ToCamel}}(_ arg: PB{{.RequestType}} = PB{{.RequestType}}()) {{if .StreamsReturns}}throws -> RPCResponseStream{{else}}-> Promise{{end}}<PB{{.ReturnsType}}> {
    return{{if .StreamsReturns}} try{{end}} self.{{if .StreamsReturns}}callStreaming{{else}}callUnary{{end}}("{{.Name | ToCamel}}", arg)
  }
  {{end}}
}
`))
}

func (g *SwiftGen) Format(path string) error {
	// TODO
	log.Println("[WARN] formatting not implemented for swift")
	return nil
}

type KotlinGen struct{}

func (g *KotlinGen) OutputPath(service *proto.Service) string {
	return path.Join(wd, "android", "app", "src", "main", "java", "gg", "strims", "ppspp", "rpc", service.Name+"Client.kt")
}

func (g *KotlinGen) Template() *template.Template {
	// TODO: don't hardcode imports
	return template.Must(template.New("ts").Funcs(funcMap).Parse(`package gg.strims.ppspp.rpc

import gg.strims.ppspp.proto.Api.*
import gg.strims.ppspp.proto.Chat.*
import gg.strims.ppspp.proto.ProfileOuterClass.*
import gg.strims.ppspp.proto.Video.*
import gg.strims.ppspp.proto.Vpn.*
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
	// check if prettier is installed
	log.Println("[WARN] formatting not implemented for kotlin")
	return nil
}
