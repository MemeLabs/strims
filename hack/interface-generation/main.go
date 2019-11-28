package main

import (
	"go/format"
	"html/template"
	"io/ioutil"
	"os"

	"gopkg.in/yaml.v2"
)

// TEMPLATE ...
const TEMPLATE string = `package {{.Package}}
{{range .Interfaces}}
type {{.Name}} interface {
{{range .Methods}} 
	{{.Name}}({{range .Inputs}} {{.}}, {{end}}) ({{range .Outputs}} {{.}}, {{end}})
{{end}}
}
{{end}}`

// Config ...
type Config struct {
	Package    string `yaml:"package"`
	Interfaces []struct {
		Name    string `yaml:"name"`
		Methods []struct {
			Name    string   `yaml:"name"`
			Type    string   `yaml:"type"`
			Inputs  []string `yaml:"inputs"`
			Outputs []string `yaml:"outputs"`
		} `yaml:"methods"`
	} `yaml:"interfaces"`
}

func main() {
	config := Config{}

	configYml, err := ioutil.ReadFile("./services.yml")
	if err != nil {
		panic(err)
	}

	if err = yaml.Unmarshal(configYml, &config); err != nil {
		panic(err)
	}

	file, err := os.Create(config.Package + ".go")
	if err != nil {
		panic(err)
	}

	t := template.Must(template.New("config").Parse(TEMPLATE))
	if err := t.Execute(file, config); err != nil {
		panic(err)
	}

	preFormat, err := ioutil.ReadFile(config.Package + ".go")
	if err != nil {
		panic(err)
	}

	formattedFile, err := format.Source(preFormat)
	if err != nil {
		panic(err)
	}

	if err = ioutil.WriteFile(config.Package+".go", formattedFile, 0777); err != nil {
		panic(err)
	}
}
