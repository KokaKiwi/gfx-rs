// Copyright 2014 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Shader parameter handling.

use std::cell::Cell;
use std::rc::Rc;
use s = device::shade;
use device::{BufferHandle, ProgramHandle, SamplerHandle, TextureHandle};

/// Helper trait to transform base types into their corresponding uniforms
pub trait ToUniform {
    /// Create a `UniformValue` representing this value.
    fn to_uniform(&self) -> s::UniformValue;
}

impl ToUniform for i32 {
    fn to_uniform(&self) -> s::UniformValue {
        s::ValueI32(*self)
    }
}

impl ToUniform for f32 {
    fn to_uniform(&self) -> s::UniformValue {
        s::ValueF32(*self)
    }
}

impl ToUniform for [i32, ..4] {
    fn to_uniform(&self) -> s::UniformValue {
        s::ValueI32Vec(*self)
    }
}

impl ToUniform for [f32, ..4] {
    fn to_uniform(&self) -> s::UniformValue {
        s::ValueF32Vec(*self)
    }
}
impl ToUniform for [[f32, ..4], ..4] {
    fn to_uniform(&self) -> s::UniformValue {
        s::ValueF32Matrix(*self)
    }
}

/// Variable index of a uniform.
pub type VarUniform = u16;

/// Variable index of a uniform block.
pub type VarBlock = u8;

/// Variable index of a texture.
pub type VarTexture = u8;

/// A texture parameter: consists of a texture handle with an optional sampler.
pub type TextureParam = (TextureHandle, Option<SamplerHandle>);

/// Borrowed parts of the `ProgramMeta`, used for data link construction
pub type ParamLinkInput<'a> = (
    &'a [s::UniformVar],
    &'a [s::BlockVar],
    &'a [s::SamplerVar]
);

/// A borrowed mutable storage for shader parameter values.
// Not sure if it's the best data structure to represent it.
pub struct ParamValues<'a> {
    /// uniform values to be provided
    pub uniforms: &'a mut [Option<s::UniformValue>],
    /// uniform buffers to be provided
    pub blocks  : &'a mut [Option<BufferHandle>],
    /// textures to be provided
    pub textures: &'a mut [Option<TextureParam>],
}

/// Encloses a shader program with its parameter
pub trait ProgramShell {
    /// Get the contained program
    fn get_program(&self) -> &ProgramHandle;
    /// Get all the contained parameter values
    fn fill_params(&self, ParamValues);
}

impl ProgramShell for ProgramHandle {
    fn get_program(&self) -> &ProgramHandle {
        self
    }

    fn fill_params(&self, params: ParamValues) {
        debug_assert!(
            params.uniforms.is_empty() &&
            params.blocks.is_empty() &&
            params.textures.is_empty(),
            "trying to bind a program that has uniforms ; please call renderer.connect_program first"
        );
    }
}

/// An error type on either the parameter storage or the program side
#[deriving(Clone, PartialEq, Show)]
pub enum ParameterError<'a> {
    /// Internal error
    ErrorInternal,
    /// Error with the named uniform
    ErrorUniform(&'a str),
    /// Error with the named uniform block
    ErrorBlock(&'a str),
    /// Error with the named texture.
    ErrorTexture(&'a str),
}

/// An error type for the link creation
#[deriving(Clone, PartialEq, Show)]
pub enum ParameterLinkError<'a> {
    /// A given parameter is not used by the program
    ErrorUnusedParameter(ParameterError<'a>),
    /// A program parameter that is not provided
    ErrorMissingParameter(ParameterError<'a>),
}

/// Abstracts the shader parameter structure, generated by the `shader_param` attribute
pub trait ShaderParam<L> {
    /// Creates a new link, self is passed as a workaround for Rust to not be lost in generics
    fn create_link(&self, ParamLinkInput) -> Result<L, ParameterError<'static>>;
    /// Get all the contained parameter values, using a given link.
    fn fill_params(&self, &L, ParamValues);
}

impl ShaderParam<()> for () {
    fn create_link(&self, (uniforms, blocks, textures): ParamLinkInput)
                   -> Result<(), ParameterError<'static>> {
        match uniforms.head() {
            Some(_) => return Err(ErrorUniform("_")),
            None => (),
        }
        match blocks.head() {
            Some(_) => return Err(ErrorBlock("_")),
            None => (),
        }
        match textures.head() {
            Some(_) => return Err(ErrorTexture("_")),
            None => (),
        }
        Ok(())
    }

    fn fill_params(&self, _: &(), _: ParamValues) {
        //empty
    }
}

/// A bundle that encapsulates a program and a custom user-provided
/// structure containing the program parameters.
/// # Type parameters:
///
/// * `L` - auto-generated structure that has a variable index for every field of T
/// * `T` - user-provided structure containing actual parameter values
#[deriving(Clone)]
pub struct CustomShell<L, T> {
    /// Shader program handle
    program: ProgramHandle,
    /// Hidden link that provides parameter indices for user data
    link: L,
    /// Global data in a user-provided struct
    pub data: T,    //TODO: move data out of the shell
}

impl<L, T: ShaderParam<L>> CustomShell<L, T> {
    /// Create a new custom shell
    pub fn new(program: ProgramHandle, link: L, data: T) -> CustomShell<L, T> {
        CustomShell {
            program: program,
            link: link,
            data: data,
        }
    }
}

impl<L, T: ShaderParam<L>> ProgramShell for CustomShell<L, T> {
    fn get_program(&self) -> &ProgramHandle {
        &self.program
    }

    fn fill_params(&self, params: ParamValues) {
        self.data.fill_params(&self.link, params);
    }
}

/// A named cell containing arbitrary value
pub struct NamedCell<T> {
    /// Name
    pub name: String,
    /// Value
    pub value: Cell<T>,
}

/// A dictionary of parameters, meant to be shared between different programs
pub struct ParamDictionary {
    /// Uniform dictionary
    pub uniforms: Vec<NamedCell<s::UniformValue>>,
    /// Block dictionary
    pub blocks: Vec<NamedCell<BufferHandle>>,
    /// Texture dictionary
    pub textures: Vec<NamedCell<TextureParam>>,
}

/// An associated link structure for `ParamDictionary` that redirects program
/// input to the relevant dictionary cell.
pub struct ParamDictionaryLink {
    uniforms: Vec<uint>,
    blocks: Vec<uint>,
    textures: Vec<uint>,
}

impl<'a> ShaderParam<ParamDictionaryLink> for &'a ParamDictionary {
    fn create_link(&self, (in_uni, in_buf, in_tex): ParamLinkInput)
                   -> Result<ParamDictionaryLink, ParameterError<'static>> {
        //TODO: proper error checks
        Ok(ParamDictionaryLink {
            uniforms: in_uni.iter().map(|var|
                self.uniforms.iter().position(|c| c.name == var.name).unwrap()
            ).collect(),
            blocks: in_buf.iter().map(|var|
                self.blocks  .iter().position(|c| c.name == var.name).unwrap()
            ).collect(),
            textures: in_tex.iter().map(|var|
                self.textures.iter().position(|c| c.name == var.name).unwrap()
            ).collect(),
        })
    }

    fn fill_params(&self, link: &ParamDictionaryLink, out: ParamValues) {
        for (&id, var) in link.uniforms.iter().zip(out.uniforms.mut_iter()) {
            *var = Some(self.uniforms[id].value.get());
        }
        for (&id, var) in link.blocks.iter().zip(out.blocks.mut_iter()) {
            *var = Some(self.blocks[id].value.get());
        }
        for (&id, var) in link.textures.iter().zip(out.textures.mut_iter()) {
            *var = Some(self.textures[id].value.get());
        }
    }
}

impl ShaderParam<ParamDictionaryLink> for Rc<ParamDictionary> {
    fn create_link(&self, input: ParamLinkInput) -> Result<ParamDictionaryLink,
                   ParameterError<'static>> {
        self.deref().create_link(input)
    }

    fn fill_params(&self, link: &ParamDictionaryLink, out: ParamValues) {
        self.deref().fill_params(link, out)
    }
}
