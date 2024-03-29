// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"fmt"
	"io"
	"log"
	"os"
	"path"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"text/template"

	"github.com/emicklei/proto"
)

var (
	generators = []Generator{&SwiftGen{}, &KotlinGen{}}
	funcMap    = make(template.FuncMap)
	wd         string
)

func ToCamel(input string) string {
	return strings.ToLower(string(input[0])) + input[1:]
}

func ImportToPascalCase(fileName string) string {
	for {
		i := strings.Index(fileName, "_")
		if i < 0 {
			break
		}
		fileName = strings.Replace(fileName, "_", "", 1)
		if len(fileName) >= i+1 {
			fileName = fileName[:i] + strings.ToUpper(string(fileName[i])) + fileName[i+1:]
		}
	}

	return strings.TrimSuffix(strings.ToUpper(string(fileName[0]))+fileName[1:], ".proto")
}

type ProtoService struct {
	*proto.Service
	Imports []*proto.Import
}

// Generator can be implemented for any language to generate client definitions
type Generator interface {
	OutputPath(ProtoService) string
	Template() *template.Template
	Format(string) error
}

func main() {
	wd, err := os.Getwd()
	if err != nil {
		log.Fatal(err)
	}
	funcMap["ToCamel"] = ToCamel
	funcMap["ToPascal"] = ImportToPascalCase

	paths := make(chan string, 128)
	var wg sync.WaitGroup

	for i := 0; i < runtime.NumCPU(); i++ {
		go func() {
			for path := range paths {
				processFile(path)
				wg.Done()
			}
		}()
	}

	err = filepath.Walk(path.Join(wd, "schema"), func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.IsDir() {
			return nil
		}

		paths <- path
		wg.Add(1)

		return nil
	})

	wg.Wait()
}

func processFile(path string) error {
	f, err := os.Open(path)
	if err != nil {
		return err
	}
	parser := proto.NewParser(f)
	res, err := parser.Parse()
	if err != nil {
		return err
	}
	var imports []*proto.Import
	proto.Walk(res, proto.WithImport(func(i *proto.Import) {
		imports = append(imports, i)
	}))

	proto.Walk(res, proto.WithService(func(s *proto.Service) {
		genService(ProtoService{s, imports})
	}))

	return nil
}

func genService(service ProtoService) {
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

func writeTemplate(t *template.Template, service ProtoService, w io.WriteCloser) error {
	if err := t.Execute(w, service); err != nil {
		return fmt.Errorf("failed to execute template: %w", err)
	}
	if err := w.Close(); err != nil {
		return fmt.Errorf("failed to close writer: %w", err)
	}
	return nil
}
