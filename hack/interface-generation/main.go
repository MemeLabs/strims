package main

import (
	"flag"
	"fmt"
	"go/format"
	"io/ioutil"
	"log"
	"os"
	"text/template"

	"golang.org/x/tools/imports"
	"gopkg.in/yaml.v2"
)

// TEMPLATE ...
const TEMPLATE string = `package {{.Package}}
{{range .Interfaces}}
type {{.Name}} interface {
{{range .Methods}}
{{.Name}}(ctx context.Context, {{.Input}}) ({{.Output}}, error)
{{end}}
}
{{end}}`

// Config ...
type Config struct {
	Package    string `yaml:"package"`
	Interfaces []struct {
		Name    string `yaml:"name"`
		Methods []struct {
			Name   string `yaml:"name"`
			Type   string `yaml:"type"`
			Input  string `yaml:"input"`
			Output string `yaml:"output"`
		} `yaml:"methods"`
	} `yaml:"interfaces"`
}

func main() {
	config := Config{}

	inputPath := flag.String("input", "services.yml", "path to the input file")
	outputPath := flag.String("output", "./", "path to output generated code")
	flag.Parse()

	configYml, err := ioutil.ReadFile(*inputPath)
	if err != nil {
		log.Fatal(fmt.Errorf("failed to open input file: %v", err))
	}

	if err = yaml.Unmarshal(configYml, &config); err != nil {
		log.Fatal(fmt.Errorf("failed to unmarshal yaml file: %v", err))
	}

	outputFilePath := *outputPath + config.Package + ".if.go"

	file, err := os.Create(outputFilePath)
	if err != nil {
		log.Fatal(fmt.Errorf("failed to create output file: %v", err))
	}
	defer file.Close()

	t := template.Must(template.New("config").Parse(TEMPLATE))
	if err := t.Execute(file, config); err != nil {
		log.Fatal(fmt.Errorf("failed to execute template: %v", err))
	}

	preFormat, err := ioutil.ReadFile(outputFilePath)
	if err != nil {
		log.Fatal(fmt.Errorf("failed to read pre-formatted file: %v", err))
	}

	formattedFile, err := format.Source(preFormat)
	if err != nil {
		log.Fatal(fmt.Errorf("failed to format generated file: %v", err))
	}

	imp, err := imports.Process(outputFilePath, formattedFile, nil)
	if err != nil {
		log.Fatal(fmt.Errorf("failed to import pkgs: %v", err))
	}

	if err = ioutil.WriteFile(outputFilePath, imp, 0777); err != nil {
		log.Fatal(fmt.Errorf("failed to write formatted file: %v", err))
	}
}
