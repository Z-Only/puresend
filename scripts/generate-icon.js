#!/usr/bin/env node

/**
 * PureSend 图标生成脚本
 * 基于 APP 主题色自动生成 1024x1024 源图标
 * 使用 pngjs 库 (无需原生编译)
 *
 * 主题色提取自 src/main.ts:
 * - primary: #1976D2 (Material Design Blue)
 *
 * 优化说明:
 * - 添加 10% 安全边距，避免图标显示过大
 * - 使用圆角矩形背景，适配 macOS 圆角风格
 * - 图标主体缩小至 70%，留出边距空间
 */

import { PNG } from 'pngjs'
import { writeFileSync, mkdirSync, existsSync } from 'fs'
import { fileURLToPath } from 'url'
import { dirname, join } from 'path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

// 从 src/main.ts 提取的主题色
const PRIMARY_COLOR = { r: 25, g: 118, b: 210 } // #1976D2
const DARK_BLUE = { r: 13, g: 71, b: 161 } // #0D47A1
const WHITE = { r: 255, g: 255, b: 255 } // #FFFFFF

// 图标设计参数
const PADDING_RATIO = 0.1 // 10% 边距
const ICON_SIZE_RATIO = 0.7 // 图标主体占 70%
const CORNER_RADIUS = 180 // 圆角半径 (适配 1024x1024)

console.log('🎨 PureSend 图标生成器 (优化版)')
console.log('================================')
console.log(
    `主色调：RGB(${PRIMARY_COLOR.r}, ${PRIMARY_COLOR.g}, ${PRIMARY_COLOR.b})`
)
console.log(
    `渐变：RGB(${PRIMARY_COLOR.r}, ${PRIMARY_COLOR.g}, ${PRIMARY_COLOR.b}) → RGB(${DARK_BLUE.r}, ${DARK_BLUE.g}, ${DARK_BLUE.b})`
)
console.log(`图标颜色：RGB(${WHITE.r}, ${WHITE.g}, ${WHITE.b})`)
console.log(`边距：${PADDING_RATIO * 100}%`)
console.log(`图标大小：${ICON_SIZE_RATIO * 100}%`)
console.log(`圆角半径：${CORNER_RADIUS}px`)
console.log('')

// 创建 1024x1024 PNG (带透明通道)
const png = new PNG({
    width: 1024,
    height: 1024,
    filterType: -1,
})

// 计算图标实际绘制区域
const padding = Math.floor(1024 * PADDING_RATIO)
const iconArea = 1024 - padding * 2
const iconScale = ICON_SIZE_RATIO

console.log('📝 绘制圆角矩形背景...')

// 绘制圆角矩形背景 (带渐变)
function drawRoundedRect(x, y, width, height, radius, colorStart, colorEnd) {
    // 使用扫描线绘制圆角矩形
    for (let cy = y; cy < y + height; cy++) {
        // 计算当前行的左右边界
        let leftX = x
        let rightX = x + width

        // 顶部圆角
        if (cy < y + radius) {
            const dy = y + radius - cy
            const dx = Math.floor(Math.sqrt(radius * radius - dy * dy))
            leftX = x + radius - dx
            rightX = x + width - radius + dx
        }
        // 底部圆角
        else if (cy > y + height - radius) {
            const dy = cy - (y + height - radius)
            const dx = Math.floor(Math.sqrt(radius * radius - dy * dy))
            leftX = x + radius - dx
            rightX = x + width - radius + dx
        }

        // 计算渐变颜色
        const ratio = (cy - y) / height
        const r = Math.round(colorStart.r + (colorEnd.r - colorStart.r) * ratio)
        const g = Math.round(colorStart.g + (colorEnd.g - colorStart.g) * ratio)
        const b = Math.round(colorStart.b + (colorEnd.b - colorStart.b) * ratio)

        // 绘制当前行
        for (let cx = leftX; cx < rightX; cx++) {
            const idx = (png.width * cy + cx) << 2
            png.data[idx] = r
            png.data[idx + 1] = g
            png.data[idx + 2] = b
            png.data[idx + 3] = 255 // Alpha
        }
    }
}

// 绘制背景圆角矩形
drawRoundedRect(
    padding,
    padding,
    iconArea,
    iconArea,
    CORNER_RADIUS,
    PRIMARY_COLOR,
    DARK_BLUE
)

// 2. 绘制纸飞机图标 (发送/传输符号)
console.log('✈️  绘制纸飞机图标...')

// 缩放后的纸飞机路径点 (缩小至 70%，居中)
const scaleCenter = (points, scale, centerX, centerY) => {
    return points.map((p) => ({
        x: Math.round(centerX + (p.x - 512) * scale),
        y: Math.round(centerY + (p.y - 512) * scale),
    }))
}

// 原始纸飞机路径点 (基于 1024x1024 中心)
const originalPaperPlanePoints = [
    { x: 200, y: 512 }, // 左端点
    { x: 824, y: 512 }, // 右端点
    { x: 612, y: 300 }, // 右上折角
    { x: 612, y: 412 }, // 右上内折
    { x: 300, y: 512 }, // 中部凹陷
    { x: 612, y: 612 }, // 右下内折
    { x: 612, y: 724 }, // 右下折角
]

// 缩放并居中
const paperPlanePoints = scaleCenter(
    originalPaperPlanePoints,
    iconScale,
    512,
    512
)

// 使用扫描线算法填充多边形
function drawFilledPolygon(points, color) {
    const minX = Math.min(...points.map((p) => p.x))
    const maxX = Math.max(...points.map((p) => p.x))
    const minY = Math.min(...points.map((p) => p.y))
    const maxY = Math.max(...points.map((p) => p.y))

    for (let y = minY; y < maxY && y < png.height; y++) {
        const intersections = []

        // 计算与扫描线的交点
        for (let i = 0; i < points.length; i++) {
            const p1 = points[i]
            const p2 = points[(i + 1) % points.length]

            if ((p1.y <= y && p2.y > y) || (p2.y <= y && p1.y > y)) {
                const x = Math.round(
                    p1.x + ((y - p1.y) / (p2.y - p1.y)) * (p2.x - p1.x)
                )
                intersections.push(x)
            }
        }

        // 排序交点并填充
        intersections.sort((a, b) => a - b)
        for (let i = 0; i < intersections.length - 1; i += 2) {
            const xStart = Math.max(intersections[i], minX)
            const xEnd = Math.min(intersections[i + 1], maxX)

            for (let x = xStart; x < xEnd && x < png.width; x++) {
                const idx = (png.width * y + x) << 2
                png.data[idx] = color.r
                png.data[idx + 1] = color.g
                png.data[idx + 2] = color.b
                png.data[idx + 3] = 255
            }
        }
    }
}

drawFilledPolygon(paperPlanePoints, WHITE)

// 3. 添加高光效果 (增强立体感)
console.log('✨ 添加高光效果...')
const highlightY = Math.floor(padding + iconArea * 0.3)
const highlightHeight = Math.floor(iconArea * 0.15)

for (let y = highlightY; y < highlightY + highlightHeight; y++) {
    const alpha = Math.floor(
        60 *
            (1 -
                Math.abs(y - (highlightY + highlightHeight / 2)) /
                    (highlightHeight / 2))
    )
    for (let x = padding; x < padding + iconArea; x++) {
        // 只在圆角矩形内部添加高光
        const cx = x - 512
        const cy = y - 512
        const dist = Math.sqrt(cx * cx + cy * cy)
        if (dist < iconArea / 2 - 20) {
            const idx = (png.width * y + x) << 2
            png.data[idx] = Math.min(255, png.data[idx] + alpha)
            png.data[idx + 1] = Math.min(255, png.data[idx + 1] + alpha)
            png.data[idx + 2] = Math.min(255, png.data[idx + 2] + alpha)
            // Alpha 保持不变
        }
    }
}

// 4. 确保输出目录存在
const outputDir = join(__dirname, '../src-tauri/icons')
if (!existsSync(outputDir)) {
    console.log(`📁 创建目录：${outputDir}`)
    mkdirSync(outputDir, { recursive: true })
}

// 5. 保存源图标 (保存为 icon-source.png 避免被 Tauri CLI 覆盖)
const outputPath = join(__dirname, '../src-tauri/icons/icon-source.png')
console.log(`💾 保存源图标：${outputPath}`)

const buffer = PNG.sync.write(png)
writeFileSync(outputPath, buffer)

console.log('')
console.log('✅ 源图标生成成功!')
console.log('')
console.log('优化内容:')
console.log('  ✓ 添加 10% 安全边距')
console.log('  ✓ 使用圆角矩形背景')
console.log('  ✓ 图标主体缩小至 70%')
console.log('  ✓ 添加高光增强立体感')
console.log('')
console.log('下一步:')
console.log('  运行以下命令生成所有平台所需的图标尺寸:')
console.log('  pnpm tauri icon src-tauri/icons/icon.png')
console.log('')
