import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Palette, Image, FileCode } from "lucide-react";

export const Appearance = () => {
    return (
        <div className="space-y-8">
            <section className="space-y-2">
                <h1 className="text-sm font-bold text-foreground">テーマ</h1>
                <div className="flex items-start gap-x-4 pb-8">
                    <div className="candidate-main">
                        <ol className="candidate-ol">
                            <li className="candidate-li" data-selected>変換候補1</li>
                            <li className="candidate-li">変換候補2</li>
                            <li className="candidate-li">変換候補3</li>
                        </ol>
                        <footer className="candidate-footer">
                            <svg width="20" height="14" viewBox="0 0 22 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                                <path d="M3.5 8C4.59202 9.04403 7.54398 10.3978 13.5068 9.93754M1.25349 5.39919C2.77722 0.413397 8.08911 0.79692 10.9673 1.24436C14.2687 1.71311 20.8969 3.82675 20.9985 8.53129C21.1255 14.412 13.1894 15.3069 10.0784 14.9233C6.96748 14.5398 -0.46071 13.0696 1.25349 5.39919Z" stroke="#838384" stroke-width="1.5" stroke-linecap="round"/>
                            </svg>
                        </footer>
                    </div>
                    <div className="border w-16 h-16 rounded-md flex items-center justify-center text-xl text-foreground">
                        あ
                    </div>
                </div>
                <div className="flex items-center space-x-4 rounded-md border p-4">
                    <Palette />
                    <div className="flex-1 space-y-1">
                        <p className="text-sm font-medium leading-none">
                            背景色
                        </p>
                    </div>
                    <div className="bg-muted-foreground w-8 h-8 rounded-full" />
                </div>
                <div className="flex items-center space-x-4 rounded-md border p-4">
                    <Image />
                    <div className="flex-1 space-y-1">
                        <p className="text-sm font-medium leading-none">
                            背景画像
                        </p>
                    </div>
                    <Button  variant="secondary">
                        画像を設定
                    </Button>
                </div>
                <div className="flex items-center space-x-4 rounded-md border p-4">
                    <Palette />
                    <div className="flex-1 space-y-1">
                        <p className="text-sm font-medium leading-none">
                            アクセントカラー
                        </p>
                    </div>
                    <div className="bg-muted-foreground w-8 h-8 rounded-full" />
                </div>
                <div className="flex items-center space-x-4 rounded-md border p-4">
                    <Palette />
                    <div className="flex-1 space-y-1">
                        <p className="text-sm font-medium leading-none">
                            テキストの色
                        </p>
                    </div>
                    <div className="bg-muted-foreground w-8 h-8 rounded-full" />
                </div>
                <div className="flex items-center space-x-4 rounded-md border p-4">
                    <FileCode />
                    <div className="flex-1 space-y-1">
                        <p className="text-sm font-medium leading-none">
                            カスタムCSS
                        </p>
                        <p className="text-xs text-muted-foreground">
                            有効にした場合、上記の設定は無視されます
                        </p>
                    </div>
                    <Switch />
                </div>
            </section>
        </div>
    )
}