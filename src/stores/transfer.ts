/**
 * 传输状态管理
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { FileMetadata, TransferTask, TransferProgress, TaskStatus } from '../types'
import {
  initTransfer,
  getTransferPort,
  prepareFileTransfer,
  sendFile,
  cancelTransfer,
  getTransferProgress,
  getActiveTasks,
  cleanupCompletedTasks,
  onTransferProgress,
  onTransferError,
  onTransferComplete
} from '../services'
import type { UnlistenFn } from '@tauri-apps/api/event'

export const useTransferStore = defineStore('transfer', () => {
  // ============ 状态 ============
  
  /** 是否已初始化 */
  const initialized = ref(false)
  
  /** 本机监听端口 */
  const listenPort = ref<number>(0)
  
  /** 活跃的传输任务 */
  const tasks = ref<Map<string, TransferTask>>(new Map())
  
  /** 当前选中的任务ID */
  const selectedTaskId = ref<string>('')
  
  /** 是否正在加载 */
  const loading = ref(false)
  
  /** 错误信息 */
  const error = ref<string>('')
  
  /** 事件监听器清理函数 */
  let unlistenFns: UnlistenFn[] = []
  
  // ============ 计算属性 ============
  
  /** 所有任务列表 */
  const taskList = computed(() => Array.from(tasks.value.values()))
  
  /** 当前选中的任务 */
  const selectedTask = computed(() => {
    if (!selectedTaskId.value) return null
    return tasks.value.get(selectedTaskId.value) || null
  })
  
  /** 正在传输的任务 */
  const transferringTasks = computed(() => 
    taskList.value.filter(t => t.status === 'transferring')
  )
  
  /** 已完成的任务 */
  const completedTasks = computed(() => 
    taskList.value.filter(t => t.status === 'completed')
  )
  
  /** 失败的任务 */
  const failedTasks = computed(() => 
    taskList.value.filter(t => t.status === 'failed')
  )
  
  /** 是否有正在传输的任务 */
  const isTransferring = computed(() => transferringTasks.value.length > 0)
  
  // ============ 方法 ============
  
  /**
   * 初始化传输服务
   */
  async function initialize() {
    if (initialized.value) return
    
    loading.value = true
    error.value = ''
    
    try {
      // 初始化传输服务
      await initTransfer()
      
      // 获取监听端口
      listenPort.value = await getTransferPort()
      
      // 同步现有任务
      const activeTasks = await getActiveTasks()
      tasks.value = new Map(activeTasks.map(t => [t.id, t]))
      
      // 注册事件监听
      unlistenFns.push(
        await onTransferProgress(handleProgress),
        await onTransferError(handleError),
        await onTransferComplete(handleComplete)
      )
      
      initialized.value = true
    } catch (e) {
      error.value = `初始化失败: ${e}`
      console.error('初始化传输服务失败:', e)
    } finally {
      loading.value = false
    }
  }
  
  /**
   * 处理进度事件
   */
  function handleProgress(progress: TransferProgress) {
    const task = tasks.value.get(progress.taskId)
    if (task) {
      task.status = progress.status
      task.progress = progress.progress
      task.transferredBytes = progress.transferredBytes
      task.speed = progress.speed
    }
  }
  
  /**
   * 处理错误事件
   */
  function handleError(progress: TransferProgress) {
    const task = tasks.value.get(progress.taskId)
    if (task) {
      task.status = 'failed'
      task.error = progress.error
    }
  }
  
  /**
   * 处理完成事件
   */
  function handleComplete(progress: TransferProgress) {
    const task = tasks.value.get(progress.taskId)
    if (task) {
      task.status = 'completed'
      task.progress = 100
      task.transferredBytes = progress.totalBytes
    }
  }
  
  /**
   * 准备文件传输
   * @param filePath 文件路径
   */
  async function prepareTransfer(filePath: string): Promise<FileMetadata | null> {
    loading.value = true
    error.value = ''
    
    try {
      const metadata = await prepareFileTransfer(filePath)
      return metadata
    } catch (e) {
      error.value = `准备传输失败: ${e}`
      console.error('准备文件传输失败:', e)
      return null
    } finally {
      loading.value = false
    }
  }
  
  /**
   * 发送文件
   * @param fileMetadata 文件元数据
   * @param peerId 目标设备ID
   * @param peerIp 目标设备IP
   * @param peerPort 目标设备端口
   */
  async function send(
    fileMetadata: FileMetadata,
    peerId: string,
    peerIp: string,
    peerPort: number
  ): Promise<string | null> {
    loading.value = true
    error.value = ''
    
    try {
      const taskId = await sendFile(fileMetadata, peerId, peerIp, peerPort)
      
      // 创建新任务并添加到列表
      const task: TransferTask = {
        id: taskId,
        file: fileMetadata,
        mode: 'local',
        peer: {
          id: peerId,
          name: peerId,
          ip: peerIp,
          port: peerPort,
          deviceType: 'unknown',
          discoveredAt: Date.now(),
          lastSeen: Date.now(),
          status: 'available'
        },
        status: 'transferring',
        progress: 0,
        transferredBytes: 0,
        speed: 0,
        createdAt: Date.now(),
        direction: 'send'
      }
      
      tasks.value.set(taskId, task)
      selectedTaskId.value = taskId
      
      return taskId
    } catch (e) {
      error.value = `发送失败: ${e}`
      console.error('发送文件失败:', e)
      return null
    } finally {
      loading.value = false
    }
  }
  
  /**
   * 取消传输
   * @param taskId 任务ID
   */
  async function cancel(taskId: string) {
    try {
      await cancelTransfer(taskId)
      
      const task = tasks.value.get(taskId)
      if (task) {
        task.status = 'cancelled'
      }
    } catch (e) {
      error.value = `取消失败: ${e}`
      console.error('取消传输失败:', e)
    }
  }
  
  /**
   * 刷新任务进度
   * @param taskId 任务ID
   */
  async function refreshProgress(taskId: string) {
    try {
      const progress = await getTransferProgress(taskId)
      const task = tasks.value.get(taskId)
      if (task) {
        task.status = progress.status
        task.progress = progress.progress
        task.transferredBytes = progress.transferredBytes
        task.speed = progress.speed
      }
    } catch (e) {
      console.error('刷新进度失败:', e)
    }
  }
  
  /**
   * 清理已完成任务
   */
  async function cleanup() {
    try {
      const count = await cleanupCompletedTasks()
      
      // 从本地列表中移除已完成的任务
      const statuses: TaskStatus[] = ['completed', 'failed', 'cancelled']
      for (const [id, task] of tasks.value) {
        if (statuses.includes(task.status)) {
          tasks.value.delete(id)
        }
      }
      
      return count
    } catch (e) {
      console.error('清理任务失败:', e)
      return 0
    }
  }
  
  /**
   * 选择任务
   * @param taskId 任务ID
   */
  function selectTask(taskId: string) {
    selectedTaskId.value = taskId
  }
  
  /**
   * 清除错误
   */
  function clearError() {
    error.value = ''
  }
  
  /**
   * 销毁 - 清理事件监听
   */
  function destroy() {
    unlistenFns.forEach(fn => fn())
    unlistenFns = []
    initialized.value = false
  }
  
  return {
    // 状态
    initialized,
    listenPort,
    tasks,
    selectedTaskId,
    loading,
    error,
    
    // 计算属性
    taskList,
    selectedTask,
    transferringTasks,
    completedTasks,
    failedTasks,
    isTransferring,
    
    // 方法
    initialize,
    prepareTransfer,
    send,
    cancel,
    refreshProgress,
    cleanup,
    selectTask,
    clearError,
    destroy
  }
})
