/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Minimal `AUGraph.h` (Audio Unit Processing Graph Services).
//!
//! Реализован граф для эмуляции iOS 2.0-4.3.5. Поддерживаются структуры
//! соединений узлов и рендер-коллбэков для полноценного отслеживания графа,
//! нужного играм вроде Plants vs Zombies. Каждая input-шина 3D Mixer-а
//! превращается в отдельный OpenAL-источник, callback дёргается из общего
//! run-loop'а через `audio_unit::render_audio_unit`.

use std::collections::HashMap;
use std::time::Instant;

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::audio_toolbox::audio_components::{
    self, AURenderCallbackStruct, AudioComponentInstance,
};
use crate::frameworks::audio_toolbox::audio_unit::{setup_audio_unit_for_render, AudioUnit};
use crate::frameworks::carbon_core::{paramErr, OSStatus};
use crate::frameworks::core_foundation::cf_run_loop::CFRunLoopGetMain;
use crate::frameworks::foundation::ns_run_loop;
use crate::mem::{guest_size_of, ConstPtr, MutPtr, SafeRead};

// =========================================================================
// MARK: - Типы
// =========================================================================

/// `AUNode` — целочисленный handle ноды в графе.
pub type AUNode = i32;

#[repr(C, packed)]
pub struct OpaqueAUGraph {
    _pad: u8,
}
unsafe impl SafeRead for OpaqueAUGraph {}

pub type AUGraph = MutPtr<OpaqueAUGraph>;

/// Описание компонента, как у `AudioComponentDescription` в `audio_components`.
#[repr(C, packed)]
#[derive(Copy, Clone, Default)]
struct ComponentDesc {
    component_type: u32,
    component_sub_type: u32,
    component_manufacturer: u32,
    component_flags: u32,
    component_flags_mask: u32,
}
unsafe impl SafeRead for ComponentDesc {}

/// Соединение между двумя узлами графа (на основе AudioUnitNodeConnection).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct AudioUnitNodeConnection {
    source_node: AUNode,
    source_output_number: u32,
    dest_node: AUNode,
    dest_input_number: u32,
}

#[derive(Default, Clone)]
struct GraphNode {
    desc: ComponentDesc,
    audio_unit: Option<AudioUnit>,
}

#[derive(Default)]
struct GraphState {
    nodes: HashMap<AUNode, GraphNode>,
    connections: Vec<AudioUnitNodeConnection>,
    next_node_id: AUNode,
    is_open: bool,
    is_initialized: bool,
    is_running: bool,
    /// Какой узел является конечным выходом (RemoteIO).
    /// Нужен, чтобы знать, какой `AudioUnit` стартовать в `AUGraphStart`.
    output_node: Option<AUNode>,
}

#[derive(Default)]
pub struct State {
    graphs: HashMap<AUGraph, GraphState>,
}
impl State {
    pub fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.au_graph
    }
}

// =========================================================================
// MARK: - Константы (типы AudioUnit'ов)
// =========================================================================

const kAudioUnitType_Output: u32 = u32::from_be_bytes(*b"auou");
const kAudioUnitSubType_RemoteIO: u32 = u32::from_be_bytes(*b"rioc");

// =========================================================================
// MARK: - Жизненный цикл графа
// =========================================================================

fn NewAUGraph(env: &mut Environment, out_graph: MutPtr<AUGraph>) -> OSStatus {
    let g: AUGraph = env.mem.alloc_and_write(OpaqueAUGraph { _pad: 0 });
    State::get(&mut env.framework_state)
        .graphs
        .insert(g, GraphState::default());
    env.mem.write(out_graph, g);
    log_dbg!("NewAUGraph() -> {:?}", g);
    0
}

fn DisposeAUGraph(env: &mut Environment, graph: AUGraph) -> OSStatus {
    log_dbg!("DisposeAUGraph({:?})", graph);
    State::get(&mut env.framework_state).graphs.remove(&graph);
    if !graph.is_null() {
        env.mem.free(graph.cast());
    }
    0
}

// =========================================================================
// MARK: - Узлы
// =========================================================================

fn AUGraphAddNode(
    env: &mut Environment,
    graph: AUGraph,
    in_desc: ConstPtr<ComponentDesc>,
    out_node: MutPtr<AUNode>,
) -> OSStatus {
    let desc = env.mem.read::<ComponentDesc, false>(in_desc);
    let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) else {
        return paramErr;
    };

    state.next_node_id += 1;
    let node_id = state.next_node_id;
    state.nodes.insert(
        node_id,
        GraphNode {
            desc,
            audio_unit: None,
        },
    );

    // Если это RemoteIO — запоминаем его как выходной узел графа.
    let is_output = desc.component_type == kAudioUnitType_Output
        && desc.component_sub_type == kAudioUnitSubType_RemoteIO;

    if is_output {
        state.output_node = Some(node_id);
    }

    env.mem.write(out_node, node_id);

    // Вытаскиваем значения в локальные переменные (копии),
    // чтобы избежать UB при взятии ссылки макросом log! из упакованной
    // структуры.
    let dt = desc.component_type;
    let ds = desc.component_sub_type;

    log_dbg!(
        "AUGraphAddNode({:?}, type=0x{:08x} sub=0x{:08x}) -> node={} (output={})",
        graph,
        dt,
        ds,
        node_id,
        is_output
    );
    0
}

fn AUGraphRemoveNode(env: &mut Environment, graph: AUGraph, node: AUNode) -> OSStatus {
    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.nodes.remove(&node);
        // Очищаем любые соединения, связанные с удаленным узлом
        state
            .connections
            .retain(|c| c.source_node != node && c.dest_node != node);
    }
    0
}

fn AUGraphCountNodes(env: &mut Environment, graph: AUGraph, out_count: MutPtr<u32>) -> OSStatus {
    let count = State::get(&mut env.framework_state)
        .graphs
        .get(&graph)
        .map(|s| s.nodes.len() as u32)
        .unwrap_or(0);
    env.mem.write(out_count, count);
    0
}

fn AUGraphGetIndNode(
    env: &mut Environment,
    graph: AUGraph,
    index: u32,
    out_node: MutPtr<AUNode>,
) -> OSStatus {
    let Some(state) = State::get(&mut env.framework_state).graphs.get(&graph) else {
        return paramErr;
    };
    let mut keys: Vec<AUNode> = state.nodes.keys().copied().collect();
    keys.sort();
    let Some(&node) = keys.get(index as usize) else {
        return paramErr;
    };
    env.mem.write(out_node, node);
    0
}

fn AUGraphNodeInfo(
    env: &mut Environment,
    graph: AUGraph,
    node: AUNode,
    out_desc: MutPtr<ComponentDesc>,
    out_audio_unit: MutPtr<AudioUnit>,
) -> OSStatus {
    let Some(state) = State::get(&mut env.framework_state).graphs.get(&graph) else {
        return paramErr;
    };
    let Some(graph_node) = state.nodes.get(&node) else {
        return paramErr;
    };
    if !out_desc.is_null() {
        env.mem.write(out_desc, graph_node.desc);
    }
    if !out_audio_unit.is_null() {
        let au = graph_node.audio_unit.unwrap_or_else(MutPtr::null);
        env.mem.write(out_audio_unit, au);
    }
    0
}

// =========================================================================
// MARK: - Open / Initialize / Start / Stop
// =========================================================================

fn AUGraphOpen(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let node_ids: Vec<AUNode> = match State::get(&mut env.framework_state).graphs.get(&graph) {
        Some(s) => s.nodes.keys().copied().collect(),
        None => return paramErr,
    };
    log_dbg!("AUGraphOpen({:?}) {} node(s)", graph, node_ids.len());

    for node_id in node_ids {
        let guest_instance: AudioComponentInstance =
            audio_components::create_audio_unit_instance(env);
        if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
            if let Some(graph_node) = state.nodes.get_mut(&node_id) {
                graph_node.audio_unit = Some(guest_instance);
            }
        }
    }

    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_open = true;
    }
    0
}

fn AUGraphClose(env: &mut Environment, graph: AUGraph) -> OSStatus {
    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_open = false;
    }
    0
}

fn AUGraphInitialize(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let units: Vec<AudioUnit> = match State::get(&mut env.framework_state).graphs.get(&graph) {
        Some(s) => s.nodes.values().filter_map(|n| n.audio_unit).collect(),
        None => return paramErr,
    };

    let run_loop = CFRunLoopGetMain(env);
    for unit in units {
        ns_run_loop::add_audio_unit(env, run_loop, unit);
    }

    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_initialized = true;
    }
    0
}

fn AUGraphUninitialize(env: &mut Environment, graph: AUGraph) -> OSStatus {
    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_initialized = false;
    }
    0
}

fn AUGraphStart(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let units: Vec<AudioUnit> = match State::get(&mut env.framework_state).graphs.get(&graph) {
        Some(s) => s.nodes.values().filter_map(|n| n.audio_unit).collect(),
        None => return paramErr,
    };
    log_dbg!("AUGraphStart({:?}) {} unit(s)", graph, units.len());

    for unit in units {
        setup_audio_unit_for_render(env, unit);
    }

    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_running = true;
    }
    0
}

fn AUGraphStop(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let units: Vec<AudioUnit> = match State::get(&mut env.framework_state).graphs.get(&graph) {
        Some(s) => s.nodes.values().filter_map(|n| n.audio_unit).collect(),
        None => return paramErr,
    };

    for unit in units {
        if let Some(obj) = audio_components::State::get(&mut env.framework_state)
            .audio_component_instances
            .get_mut(&unit)
        {
            obj.started = false;
        }
    }

    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_running = false;
    }
    0
}

fn AUGraphIsOpen(env: &mut Environment, graph: AUGraph, out_is_open: MutPtr<u8>) -> OSStatus {
    let v = State::get(&mut env.framework_state)
        .graphs
        .get(&graph)
        .map(|s| s.is_open)
        .unwrap_or(false);
    env.mem.write(out_is_open, v as u8);
    0
}

fn AUGraphIsInitialized(env: &mut Environment, graph: AUGraph, out_v: MutPtr<u8>) -> OSStatus {
    let v = State::get(&mut env.framework_state)
        .graphs
        .get(&graph)
        .map(|s| s.is_initialized)
        .unwrap_or(false);
    env.mem.write(out_v, v as u8);
    0
}

fn AUGraphIsRunning(env: &mut Environment, graph: AUGraph, out_v: MutPtr<u8>) -> OSStatus {
    let v = State::get(&mut env.framework_state)
        .graphs
        .get(&graph)
        .map(|s| s.is_running)
        .unwrap_or(false);
    env.mem.write(out_v, v as u8);
    0
}

// =========================================================================
// MARK: - Connections / Callbacks
// =========================================================================

fn AUGraphConnectNodeInput(
    env: &mut Environment,
    graph: AUGraph,
    src_node: AUNode,
    src_output_number: u32,
    dest_node: AUNode,
    dest_input_number: u32,
) -> OSStatus {
    log_dbg!(
        "AUGraphConnectNodeInput({:?}): src_node={} out={} -> dest_node={} in={}",
        graph,
        src_node,
        src_output_number,
        dest_node,
        dest_input_number
    );

    let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) else {
        return paramErr;
    };

    state.connections.push(AudioUnitNodeConnection {
        source_node: src_node,
        source_output_number: src_output_number,
        dest_node,
        dest_input_number,
    });

    0
}

fn AUGraphDisconnectNodeInput(
    env: &mut Environment,
    graph: AUGraph,
    dest_node: AUNode,
    dest_input_number: u32,
) -> OSStatus {
    let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) else {
        return paramErr;
    };

    state
        .connections
        .retain(|c| !(c.dest_node == dest_node && c.dest_input_number == dest_input_number));

    log_dbg!(
        "AUGraphDisconnectNodeInput({:?}): dest_node={} in={}",
        graph,
        dest_node,
        dest_input_number
    );
    0
}

fn AUGraphSetNodeInputCallback(
    env: &mut Environment,
    graph: AUGraph,
    dest_node: AUNode,
    dest_input_number: u32,
    in_input_callback: ConstPtr<AURenderCallbackStruct>,
) -> OSStatus {
    let cb = env
        .mem
        .read::<AURenderCallbackStruct, false>(in_input_callback);
    let dest_unit: Option<AudioUnit> = State::get(&mut env.framework_state)
        .graphs
        .get(&graph)
        .and_then(|s| s.nodes.get(&dest_node))
        .and_then(|n| n.audio_unit);

    let Some(dest_unit) = dest_unit else {
        return paramErr;
    };

    let proc_copy = cb.input_proc;
    let ref_con_copy = cb.input_proc_ref_con;

    if let Some(obj) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&dest_unit)
    {
        let bus = obj.mixer_buses.entry(dest_input_number).or_default();
        bus.render_callback = Some(cb);
        if bus.last_render_time.is_none() {
            bus.last_render_time = Some(Instant::now());
        }
    }

    log_dbg!(
        "AUGraphSetNodeInputCallback({:?}, dest_node={}, bus={}, proc={:?}, ref_con={:?}) -> unit={:?}",
        graph, dest_node, dest_input_number, proc_copy, ref_con_copy, dest_unit
    );
    0
}

fn AUGraphUpdate(env: &mut Environment, _graph: AUGraph, out_is_updated: MutPtr<u8>) -> OSStatus {
    if !out_is_updated.is_null() {
        env.mem.write(out_is_updated, 1);
    }
    0
}

fn AUGraphAddRenderNotify(
    _env: &mut Environment,
    _graph: AUGraph,
    _proc: ConstPtr<u8>,
    _ref_con: ConstPtr<u8>,
) -> OSStatus {
    0
}

fn AUGraphRemoveRenderNotify(
    _env: &mut Environment,
    _graph: AUGraph,
    _proc: ConstPtr<u8>,
    _ref_con: ConstPtr<u8>,
) -> OSStatus {
    0
}

// =========================================================================
// MARK: - Экспорт функций
// =========================================================================

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(NewAUGraph(_)),
    export_c_func!(DisposeAUGraph(_)),
    export_c_func!(AUGraphAddNode(_, _, _)),
    export_c_func!(AUGraphRemoveNode(_, _)),
    export_c_func!(AUGraphCountNodes(_, _)),
    export_c_func!(AUGraphGetIndNode(_, _, _)),
    export_c_func!(AUGraphNodeInfo(_, _, _, _)),
    export_c_func!(AUGraphOpen(_)),
    export_c_func!(AUGraphClose(_)),
    export_c_func!(AUGraphInitialize(_)),
    export_c_func!(AUGraphUninitialize(_)),
    export_c_func!(AUGraphStart(_)),
    export_c_func!(AUGraphStop(_)),
    export_c_func!(AUGraphIsOpen(_, _)),
    export_c_func!(AUGraphIsInitialized(_, _)),
    export_c_func!(AUGraphIsRunning(_, _)),
    export_c_func!(AUGraphConnectNodeInput(_, _, _, _, _)),
    export_c_func!(AUGraphDisconnectNodeInput(_, _, _)),
    export_c_func!(AUGraphSetNodeInputCallback(_, _, _, _)),
    export_c_func!(AUGraphUpdate(_, _)),
    export_c_func!(AUGraphAddRenderNotify(_, _, _)),
    export_c_func!(AUGraphRemoveRenderNotify(_, _, _)),
];

// Предотвращаем "unused" предупреждение для guest_size_of import.
#[allow(dead_code)]
const _SIZE_PROBE: usize = guest_size_of::<OpaqueAUGraph>() as usize;
