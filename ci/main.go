package main

import (
	"context"
	"fmt"
	"os"

	"dagger.io/dagger"
)

func main() {
	ctx := context.Background()

	// initialize Dagger client
	client, err := dagger.Connect(ctx, dagger.WithLogOutput(os.Stderr))
	if err != nil {
		panic(err)
	}
	defer client.Close()

	// get reference to the local project
	src := client.Host().Directory(".")

	// get `rust` image
	rust := client.Container().From("rust:1.75")

	// mount cloned repository into `rust` image
	rust = rust.WithDirectory("/src", src).WithWorkdir("/src")

	// define the application build
	path := "target/release/merlin"
	rust = rust.WithExec([]string{"cargo", "build", "--release"})

	// get reference to build output directory in container
	output := rust.Directory("/src/target/release")

	// write contents of container build/ directory to the host
	_, err = output.Export(ctx, "./build")
	if err != nil {
		panic(err)
	}

	// run tests
	test := rust.WithExec([]string{"cargo", "test"})
	out, err := test.Stdout(ctx)
	if err != nil {
		panic(err)
	}
	fmt.Println("Tests output:", out)

	// run clippy for linting
	lint := rust.WithExec([]string{"cargo", "clippy", "--", "-D", "warnings"})
	lintOut, err := lint.Stdout(ctx)
	if err != nil {
		panic(err)
	}
	fmt.Println("Clippy output:", lintOut)

	// check formatting
	fmt := rust.WithExec([]string{"cargo", "fmt", "--check"})
	fmtOut, err := fmt.Stdout(ctx)
	if err != nil {
		panic(err)
	}
	fmt.Println("Format check output:", fmtOut)

	fmt.Printf("Application built successfully at %s\n", path)
}
