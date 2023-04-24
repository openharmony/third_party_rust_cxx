CXX &mdash; Rust和C++之间的安全FFI
=========================================

[<img alt="github" src="https://img.shields.io/badge/github-dtolnay/CXX-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/dtolnay/CXX)
[<img alt="crates.io" src="https://img.shields.io/crates/v/CXX.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/CXX)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-CXX-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/cxx)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/dtolnay/CXX/ci.yml" height="20">](https://github.com/dtolnay/CXX)


## 概述


CXX工具提供了一种安全的互相调用机制，可以实现rust和C++的互相调用，而不会像使用bindgen或cbindgen那样，生成不安全的C风格的绑定时，可能会出现很多问题。但是，这并不能改变C++代码100%是不安全的事实。当检查一个项目的安全性时，需要负责审核所有不安全的Rust代码和所有的C++代码。这种安全理念下的检查思想是，只对C++端进行检查，就足以发现所有不安全问题，也就是说，Rust方面被认为是100%安全的。

CXX通过FFI（Foreign Function Interface）和函数签名的形式来实现接口和类型声明，并对类型和函数签名进行静态分析，以维护Rust和C++的不变量和要求。

如果所有的静态分析都通过了，那么CXX就会使用一对代码生成器来生成两侧相关的`extern "C"`签名，以及任何必要的静态断言，以便在以后的构建过程中验证正确性。在Rust侧，这个代码生成器只是一个属性宏。在C++方面，它可以是一个小型的Cargo构建脚本或者别的构建系统，如Bazel或Buck，CXX也提供了一个命令行工具来生成头文件和源文件，容易集成。

产生的FFI桥的运行成本为零或可忽略不计，也就是说，没有复制，没有序列化，没有内存分配，也不需要运行时检查。

FFI的签名能够使用来自任何一方的原生类型、比如Rust的`String`或C++的`std::string`，Rust的`Box`或C++的`std::unique_ptr`，Rust的`Vec`或C++的`std::vector`，等等的任意组合。CXX保证ABI兼容性，基于关键的标准库类型的内置绑定，在这些类型上向另一种语言暴露API。这些类型有着明确的对应关系，例如，当从Rust操作一个C++字符串时的时候，它的`len()`方法就变成了对C++定义的`size()`成员函数的调用。当从C++操作一个Rust字符串时，其`size()`成员函数函数调用Rust的`len()`。

<br>

## 使用指导

建议初学者参考CXX官网指导 **<https://cxx.rs>**，了解具体CXX的使用方法，然后在build仓的rust/test目录下具体使用CXX来实现接口转换。其中test_cxx介绍了C++调用rust的示例，test_cxx_rust介绍了rust调用C++的示例。

<br>


## 细节

FFI边界的语言涉及3种类型的字段：

- **共享结构**  
该字段对两种语言都是可见的。

- **不透明类型**  
该字段对另一种语言是保密的。这些类型不能通过FFI的值来传递，而只能在间接传递，比如一个引用`&`，一个Rust`Box`，或者一个`UniquePtr`。可以是一个类型别名可以是一个类型别名，用于任意复杂的通用语言特定类型，这取决于自己写的用例。

- **函数**  
在任一语言中实现，可从另一语言中调用。

<br>

## 与bindgen的对比

bindgen主要用来实现rust代码对c接口的单向调用；CXX工具可以实现rust和C++的互相调用。

<br>

## 基于cargo的构建

对于由Cargo的构建，需要使用一个构建脚本来运行CXX的C++代码生成器。

典型的构建脚本如下：

[`cc::Build`]: https://docs.rs/cc/1.0/cc/struct.Build.html

```toml
# Cargo.toml

[build-dependencies]
CXX-build = "1.0"
```

```rust
// build.rs

fn main() {
    CXX_build::bridge("src/main.rs")  // returns a cc::Build
        .file("src/demo.cc")
        .flag_if_supported("-std=C++11")
        .compile("cxxbridge-demo");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/demo.cc");
    println!("cargo:rerun-if-changed=include/demo.h");
}
```

<br>

## 基于非cargo的构建

对于在非Cargo构建中的使用，如Bazel或Buck，CXX提供了另一种方式产生C++侧的头文件和源代码文件，作为一个独立的命令行工具使用。

```bash
$ cargo install cxxbridge-cmd
$ cxxbridge src/main.rs --header > path/to/mybridge.h
$ cxxbridge src/main.rs > path/to/mybridge.cc
```

<br>

## 安全性

确保安全性需要考虑以下内容：

- 设计让配对代码生成器一起工作，控制FFI边界的两边。通常情况下，在Rust中编写自己的`extern "C"`块是是不安全的，因为Rust编译器没有办法知道每个开发者的签名是否真的与别的语言实现的签名相匹配。有了CXX，就可以实现这种可见性，并且知道另一边是什么的内容。

- 静态分析可以检测并防止不应该以值传递的类型从C++到Rust中以值传递的类型，例如，因为它们可能包含内部指针，而这些指针会被Rust的移动行为所破坏。

- 令人惊讶的是，Rust中的结构和C++中的结构的布局/字段/对齐方式/一切都完全相同、但在通过值传递时，仍然不是相同的ABI。这是一个长期存在的bindgen的错误，导致看起来正确的代码出现segfaults([issue_778](https://github.com/rust-lang/rust-bindgen/issues/778))。CXX知道这一点，并且可以插入必要的零成本的解决方法，所以请继续并毫无顾虑地传递开发者写的结构。这可以通过拥有边界的两边，而不是只有一边。

- 模板实例化：例如，为了在Rust中展示一个`UniquePtr<T>`类型，该类型由一个真正的C++的`unique_ptr`支持。为了在Rust中展示一个由真正的C++`unique_ptr`的指针类型，可以使用Rust trait来将行为连接到由别的语言执行的模板实例上。


<br>

## 内置类型

除了所有的原生类型（i32 &lt;=&gt; int32_t）之外，还有以下常见类型可用于共享结构的字段以及函数的参数和返回值。

<table>
<tr><th>name in Rust</th><th>name in C++</th><th>restrictions</th></tr>
<tr><td>String</td><td>rust::String</td><td></td></tr>
<tr><td>&amp;str</td><td>rust::Str</td><td></td></tr>
<tr><td>&amp;[T]</td><td>rust::Slice&lt;const T&gt;</td><td><sup><i>cannot hold opaque C++ type</i></sup></td></tr>
<tr><td>&amp;mut [T]</td><td>rust::Slice&lt;T&gt;</td><td><sup><i>cannot hold opaque C++ type</i></sup></td></tr>
<tr><td><a href="https://docs.rs/cxx/1.0/CXX/struct.CXXString.html">CXXString</a></td><td>std::string</td><td><sup><i>cannot be passed by value</i></sup></td></tr>
<tr><td>Box&lt;T&gt;</td><td>rust::Box&lt;T&gt;</td><td><sup><i>cannot hold opaque C++ type</i></sup></td></tr>
<tr><td><a href="https://docs.rs/cxx/1.0/CXX/struct.UniquePtr.html">UniquePtr&lt;T&gt;</a></td><td>std::unique_ptr&lt;T&gt;</td><td><sup><i>cannot hold opaque Rust type</i></sup></td></tr>
<tr><td><a href="https://docs.rs/cxx/1.0/CXX/struct.SharedPtr.html">SharedPtr&lt;T&gt;</a></td><td>std::shared_ptr&lt;T&gt;</td><td><sup><i>cannot hold opaque Rust type</i></sup></td></tr>
<tr><td>[T; N]</td><td>std::array&lt;T, N&gt;</td><td><sup><i>cannot hold opaque C++ type</i></sup></td></tr>
<tr><td>Vec&lt;T&gt;</td><td>rust::Vec&lt;T&gt;</td><td><sup><i>cannot hold opaque C++ type</i></sup></td></tr>
<tr><td><a href="https://docs.rs/cxx/1.0/CXX/struct.CXXVector.html">CXXVector&lt;T&gt;</a></td><td>std::vector&lt;T&gt;</td><td><sup><i>cannot be passed by value, cannot hold opaque Rust type</i></sup></td></tr>
<tr><td>*mut T, *const T</td><td>T*, const T*</td><td><sup><i>fn with a raw pointer argument must be declared unsafe to call</i></sup></td></tr>
<tr><td>fn(T, U) -&gt; V</td><td>rust::Fn&lt;V(T, U)&gt;</td><td><sup><i>only passing from Rust to C++ is implemented so far</i></sup></td></tr>
<tr><td>Result&lt;T&gt;</td><td>throw/catch</td><td><sup><i>allowed as return type only</i></sup></td></tr>
</table>

`rust`命名空间的C++ API是由*include/CXX.h*文件定义的。使用这些类型时种类的时候，需要C++代码中包含这个头文件。

以下类型很快被支持，只是还没有实现。

<table>
<tr><th>name in Rust</th><th>name in C++</th></tr>
<tr><td>BTreeMap&lt;K, V&gt;</td><td><sup><i>tbd</i></sup></td></tr>
<tr><td>HashMap&lt;K, V&gt;</td><td><sup><i>tbd</i></sup></td></tr>
<tr><td>Arc&lt;T&gt;</td><td><sup><i>tbd</i></sup></td></tr>
<tr><td>Option&lt;T&gt;</td><td><sup><i>tbd</i></sup></td></tr>
<tr><td><sup><i>tbd</i></sup></td><td>std::map&lt;K, V&gt;</td></tr>
<tr><td><sup><i>tbd</i></sup></td><td>std::unordered_map&lt;K, V&gt;</td></tr>
</table>

<br>

## 剩余工作

当前CXX工具还没有达到普遍使用阶段，在使用该工具的过程中有任何问题欢迎开发者在社区反馈。

<br>
