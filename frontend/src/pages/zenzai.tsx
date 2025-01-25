import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
import { Bot, User, Cpu } from "lucide-react";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select"

export const Zenzai = () => {
    return (
        <div className="space-y-8">
            <section className="space-y-2">
                <h1 className="text-sm font-bold text-foreground">Zenzai</h1>
                <div className="flex items-center space-x-4 rounded-md border p-4">
                    <Bot />
                    <div className="flex-1 space-y-1">
                        <p className="text-sm font-medium leading-none">
                            Zenzaiを有効化
                        </p>
                        <p className="text-xs text-muted-foreground">
                            Zenzaiを有効にして、変換精度を向上させます
                        </p>
                    </div>
                    <Switch />
                </div>
                <div className="space-y-4 rounded-md border p-4">
                    <div className="flex items-center space-x-4 ">
                        <User />
                        <div className="flex-1 space-y-1">
                            <p className="text-sm font-medium leading-none">
                                変換プロファイル
                            </p>
                            <p className="text-xs text-muted-foreground">
                                Zenzaiで利用されるユーザー情報です。あなたについての情報を書いてください。
                            </p>
                        </div>
                    </div>
                    <Textarea placeholder="山田太郎、数学科の学生。" />
                </div>
                <div className="flex items-center space-x-4 rounded-md border p-4">
                    <Cpu />
                    <div className="flex-1 space-y-1">
                        <p className="text-sm font-medium leading-none">
                            バックエンド
                        </p>
                        <p className="text-xs text-muted-foreground">
                            Zenzaiを利用するバックエンドを選択します
                        </p>
                    </div>
                    <Select>
                        <SelectTrigger className="w-48">
                            <SelectValue placeholder="バックエンドを選択" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="cpu">CPU (非推奨)</SelectItem>
                            <SelectItem value="cuda">CUDA (NVIDIA GPU)</SelectItem>
                            <SelectItem value="HIP">HIP (AMD GPU)</SelectItem>
                        </SelectContent>
                    </Select>
                </div>
            </section>
        </div>
    )
}