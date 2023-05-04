CXX &mdash; Rust和C++之间的安全FFI
=========================================

[<img alt="github" src="https://img.shields.io/badge/github-dtolnay/CXX-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/dtolnay/CXX)
[<img alt="crates.io" src="https://img.shields.io/crates/v/CXX.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/CXX)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-CXX-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/cxx)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/dtolnay/CXX/ci.yml" height="20">](https://github.com/dtolnay/CXX)


## 引入背景


CXX工具提供了一种安全的互相调用机制，可以实现rust和C++的互相调用。

CXX通过FFI（Foreign Function Interface）和函数签名的形式来实现接口和类型声明，并对类型和函数签名进行静态分析，以维护Rust和C++的不变量和要求。

<br>

## CXX工具在OH上的使用指导

### C++调用Rust接口

1. 在Rust侧文件lib.rs里mod ffi写清楚需要调用的C++接口，并将接口包含在extern "Rust"里面，暴露给C++侧使用。

   ```rust
   //! #[cxx::bridge]
   #[cxx::bridge]
   mod ffi{
       #![allow(dead_code)]
       #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
       struct Shared {
           z: usize,
       }
       extern "Rust"{
           fn print_message_in_rust();
           fn r_return_primitive() -> usize;
           fn r_return_shared() -> Shared;
           fn r_return_rust_string() -> String;
           fn r_return_sum(_: usize, _: usize) -> usize;
       }
   }

   fn print_message_in_rust(){
       println!("Here is a test for cpp call Rust.");
   }
   fn r_return_shared() -> ffi::Shared {
       println!("Here is a message from Rust,test for ffi::Shared:");
       ffi::Shared { z: 1996 }
   }
   fn r_return_primitive() -> usize {
       println!("Here is a message from Rust,test for usize:");
       1997
   }
   fn r_return_rust_string() -> String {
       println!("Here is a message from Rust,test for String");
       "Hello World!".to_owned()
   }
   fn r_return_sum(n1: usize, n2: usize) -> usize {
       println!("Here is a message from Rust,test for {} + {} is:",n1 ,n2);
       n1 + n2
   }

   ```

2. C++侧将cxx工具转换出来的lib.rs.h包含进来，就可以使用C++侧的接口。

   ```c++
   #include <iostream>
   #include "build/rust/tests/test_cxx/src/lib.rs.h"

   int main(int argc, const char* argv[])
   {
       int a = 2021;
       int b = 4;
       print_message_in_rust();
       std::cout << r_return_primitive() << std::endl;
       std::cout << r_return_shared().z << std::endl;
       std::cout << std::string(r_return_rust_string()) << std::endl;
       std::cout << r_return_sum(a, b) << std::endl;
       return 0;
   }
   ```

3. 添加构建文件BUILD.gn。rust_cxx底层调用CXX工具将lib.rs文件转换成lib.rs.h和lib.rs.cc文件，ohos_rust_static_ffi实现Rust侧源码的编译，ohos_executable实现C++侧代码的编译。

   ```
   import("//build/ohos.gni")
   import("//build/templates/rust/rust_cxx.gni")

   rust_cxx("test_cxx_exe_gen") {
       sources = [ "src/lib.rs" ]
   }

   ohos_rust_static_ffi("test_cxx_examp_rust") {
       sources = [ "src/lib.rs" ]
       deps = [ "//build/rust:cxx_rustdeps" ]
   }

   ohos_executable("test_cxx_exe") {
       sources = [ "main.cpp" ]
       sources += get_target_outputs(":test_cxx_exe_gen")

       include_dirs = [ "${target_gen_dir}" ]
       deps = [
       ":test_cxx_examp_rust",
       ":test_cxx_exe_gen",
       "//build/rust:cxx_cppdeps",
       ]
   }
   ```

**调测验证**
![cpp_call_rust](./cpp_call_rust.png)


### Rust调用C++

1. 添加头文件client_blobstore.h。

   ```c++
   #ifndef BUILD_RUST_TESTS_CLIENT_BLOBSTORE_H
   #define BUILD_RUST_TESTS_CLIENT_BLOBSTORE_H
   #include <memory>
   #include "third_party/rust/cxx/include/cxx.h"

   namespace nsp_org {
   namespace nsp_blobstore {
   struct MultiBufs;
   struct Metadata_Blob;

   class client_blobstore {
   public:
       client_blobstore();
       uint64_t put_buf(MultiBufs &buf) const;
       void add_tag(uint64_t blobid, rust::Str add_tag) const;
       Metadata_Blob get_metadata(uint64_t blobid) const;

   private:
       class impl;
       std::shared_ptr<impl> impl;
   };

   std::unique_ptr<client_blobstore> blobstore_client_new();
   } // namespace nsp_blobstore
   } // namespace nsp_org
   #endif
   ```

2. 添加cpp文件client_blobstore.cpp。

   ```c++
   #include <algorithm>
   #include <functional>
   #include <set>
   #include <string>
   #include <unordered_map>
   #include "src/main.rs.h"
   #include "build/rust/tests/test_cxx_rust/include/client_blobstore.h"

   namespace nsp_org {
   namespace nsp_blobstore {
   // Toy implementation of an in-memory nsp_blobstore.
   //
   // In reality the implementation of client_blobstore could be a large complex C++
   // library.
   class client_blobstore::impl {
       friend client_blobstore;
       using Blob = struct {
           std::string data;
           std::set<std::string> tags;
       };
       std::unordered_map<uint64_t, Blob> blobs;
   };

   client_blobstore::client_blobstore() : impl(new class client_blobstore::impl) {}

   // Upload a new blob and return a blobid that serves as a handle to the blob.
   uint64_t client_blobstore::put_buf(MultiBufs &buf) const
   {
       std::string contents;

       // Traverse the caller's res_chunk iterator.
       //
       // In reality there might be sophisticated batching of chunks and/or parallel
       // upload implemented by the nsp_blobstore's C++ client.
       while (true) {
           auto res_chunk = next_chunk(buf);
           if (res_chunk.size() == 0) {
           break;
           }
           contents.append(reinterpret_cast<const char *>(res_chunk.data()), res_chunk.size());
       }

       // Insert into map and provide caller the handle.
       auto res = std::hash<std::string> {} (contents);
       impl->blobs[res] = {std::move(contents), {}};
       return res;
   }

   // Add add_tag to an existing blob.
   void client_blobstore::add_tag(uint64_t blobid, rust::Str add_tag) const
   {
       impl->blobs[blobid].tags.emplace(add_tag);
   }

   // Retrieve get_metadata about a blob.
   Metadata_Blob client_blobstore::get_metadata(uint64_t blobid) const
   {
       Metadata_Blob get_metadata {};
       auto blob = impl->blobs.find(blobid);
       if (blob != impl->blobs.end()) {
           get_metadata.size = blob->second.data.size();
           std::for_each(blob->second.tags.cbegin(), blob->second.tags.cend(),
               [&](auto &t) { get_metadata.tags.emplace_back(t); });
       }
       return get_metadata;
   }

   std::unique_ptr<client_blobstore> blobstore_client_new()
   {
       return std::make_unique<client_blobstore>();
   }
   } // namespace nsp_blobstore
   } // namespace nsp_org

   ```

3. main.rs文件，在main.rs文件的ffi里面，通过宏include!将头文件client_blobstore.h引入进来，从而在Rust的main函数里面就可以通过ffi的方式调用C++的接口。

   ```rust
   //! test_cxx_rust
   #[cxx::bridge(namespace = "nsp_org::nsp_blobstore")]
   mod ffi {
       // Shared structs with fields visible to both languages.
       struct Metadata_Blob {
           size: usize,
           tags: Vec<String>,
       }

       // Rust types and signatures exposed to C++.
       extern "Rust" {
           type MultiBufs;

           fn next_chunk(buf: &mut MultiBufs) -> &[u8];
       }

       // C++ types and signatures exposed to Rust.
       unsafe extern "C++" {
           include!("build/rust/tests/test_cxx_rust/include/client_blobstore.h");

           type client_blobstore;

           fn blobstore_client_new() -> UniquePtr<client_blobstore>;
           fn put_buf(&self, parts: &mut MultiBufs) -> u64;
           fn add_tag(&self, blobid: u64, add_tag: &str);
           fn get_metadata(&self, blobid: u64) -> Metadata_Blob;
       }
   }

   // An iterator over contiguous chunks of a discontiguous file object.
   //
   // Toy implementation uses a Vec<Vec<u8>> but in reality this might be iterating
   // over some more complex Rust data structure like a rope, or maybe loading
   // chunks lazily from somewhere.
   /// pub struct MultiBufs
   pub struct MultiBufs {
       chunks: Vec<Vec<u8>>,
       pos: usize,
   }
   /// pub fn next_chunk
   pub fn next_chunk(buf: &mut MultiBufs) -> &[u8] {
       let next = buf.chunks.get(buf.pos);
       buf.pos += 1;
       next.map_or(&[], Vec::as_slice)
   }

   /// fn main()
   fn main() {
       let client = ffi::blobstore_client_new();

       // Upload a blob.
       let chunks = vec![b"fearless".to_vec(), b"concurrency".to_vec()];
       let mut buf = MultiBufs { chunks, pos: 0 };
       let blobid = client.put_buf(&mut buf);
       println!("This is a test for Rust call cpp:");
       println!("blobid = {}", blobid);

       // Add a add_tag.
       client.add_tag(blobid, "rust");

       // Read back the tags.
       let get_metadata = client.get_metadata(blobid);
       println!("tags = {:?}", get_metadata.tags);
   }
   ```

4. 添加构建文件BUILD.gn。使用CXX将main.rs转换成lib.rs.h和lib.rs.cc，同时将产物作为test_cxx_rust_staticlib的源码，编译Rust源码main.rs并将test_cxx_rust_staticlib依赖进来。

   ```
   import("//build/ohos.gni")

   rust_cxx("test_cxx_rust_gen") {
     sources = [ "src/main.rs" ]
   }

   ohos_static_library("test_cxx_rust_staticlib") {
     sources = [ "src/client_blobstore.cpp" ]
     sources += get_target_outputs(":test_cxx_rust_gen")
     include_dirs = [
       "${target_gen_dir}",
       "//third_party/rust/cxx/v1/crate/include",
       "include",
     ]
     deps = [
       ":test_cxx_rust_gen",
       "//build/rust:cxx_cppdeps",
     ]
   }

   ohos_rust_executable("test_cxx_rust") {
     sources = [ "src/main.rs" ]
     deps = [
       ":test_cxx_rust_staticlib",
       "//build/rust:cxx_rustdeps",
     ]
   }
   ```

**调测验证**
![rust_call_cpp](./rust_call_cpp.png)

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

## 开发者贡献

当前CXX工具还没有达到普遍使用阶段，在使用该工具的过程中有任何问题欢迎开发者在社区issue中反馈。

<br>
