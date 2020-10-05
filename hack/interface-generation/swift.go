package main

import (
	"log"
	"path"
	"text/template"
)

type SwiftGen struct{}

func (g *SwiftGen) OutputPath(service ProtoService) string {
	return path.Join(wd, "ios", "App", "App", "API", service.Name+"Client.swift")
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
    return{{if .StreamsReturns}} try{{end}} self.{{if .StreamsReturns}}callStreaming{{else}}callUnary{{end}}("{{$.Name}}/{{.Name | ToPascal}}", arg)
  }
  {{end}}
}
`))
}

func (g *SwiftGen) Format(path string) error {
	log.Println("[WARN] formatting not implemented for swift")
	return nil
}
