import 'package:flutter/material.dart';
import 'package:e2xf/src/rust/frb_generated.dart';
import 'main_viewmodel.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(MainApp(viewModel: MainViewModel()));
}

class MainApp extends StatefulWidget {
  const MainApp({super.key, required this.viewModel});
  final MainViewModel viewModel;

  @override
  State<MainApp> createState() => _MainAppState();
}

class _MainAppState extends State<MainApp> {
  final int _marginValue = 20;
  final double _radiusValue = 8;

  SizedBox _cfg() {
    return SizedBox(
      height: 200,
      child: TextField(
        controller: widget.viewModel.cfgController,
        maxLines: null,
        expands: true,
        decoration: InputDecoration(
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(_radiusValue),
          ),
          hintText: '编辑配置内容',
        ),
      ),
    );
  }

  Row _item(String? path, String hint, String btText, VoidCallback onPressed) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.start,
      children: [
        Expanded(
          child: Container(
            width: double.infinity,
            constraints: const BoxConstraints(minHeight: 50),
            padding: const EdgeInsets.only(
              left: 16.0,
              right: 16.0,
              top: 8.0,
              bottom: 8.0,
            ),
            decoration: BoxDecoration(
              color: Colors.grey[100],
              borderRadius: BorderRadius.circular(_radiusValue),
              border: Border.all(color: Colors.grey[300]!),
            ),
            child: Align(
              alignment: Alignment.centerLeft,
              child: Text(
                path ?? hint,

                style: const TextStyle(
                  fontSize: 16,
                  fontWeight: FontWeight.normal,
                ),
                textAlign: TextAlign.left,
              ),
            ),
          ),
        ),
        const SizedBox(width: 20),
        ElevatedButton(
          style: ElevatedButton.styleFrom(
            minimumSize: const Size(180, 50),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(_radiusValue), // 设置圆角半径为 8
            ),
          ),
          onPressed: onPressed,
          child: Text(btText),
        ),
      ],
    );
  }

  Expanded _logText() {
    return Expanded(
      flex: 1,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.all(8.0),
        decoration: BoxDecoration(
          color: Colors.white,
          borderRadius: BorderRadius.circular(_radiusValue),
          border: Border.all(color: Colors.grey[300]!),
        ),
        child: SingleChildScrollView(
          child: Text(
            widget.viewModel.log,
            style: const TextStyle(
              fontSize: 14,
              fontFamily: 'monospace', // 等宽字体，适合日志
              fontWeight: FontWeight.normal,
            ),
            textAlign: TextAlign.left,
          ),
        ),
      ),
    );
  }

  SizedBox _marginTop(int value) {
    return SizedBox(height: value.toDouble());
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        body: Padding(
          padding: const EdgeInsets.all(16.0),
          child: ListenableBuilder(
            listenable: widget.viewModel,
            builder: (context, child) {
              return Column(
                mainAxisAlignment: MainAxisAlignment.start,
                children: [
                  _cfg(),
                  _item(
                    widget.viewModel.selectedJsonPath,
                    'JSON文件未选择',
                    '选择JSON文件',
                    () => widget.viewModel.selectJsonFile(),
                  ),
                  _marginTop(_marginValue),
                  _item(
                    widget.viewModel.selectedExcelPath,
                    'Excel文件未选择',
                    '选择Excel文件',
                    () => widget.viewModel.selectExcelFile(),
                  ),
                  _marginTop(_marginValue),
                  _item(
                    widget.viewModel.selectedXmlFolderPath,
                    '模块文件夹未选择',
                    '选择模块文件夹',
                    () => widget.viewModel.selectFolder(),
                  ),
                  _marginTop(_marginValue),
                  ElevatedButton(
                    style: ElevatedButton.styleFrom(
                      minimumSize: Size(double.infinity, 50),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(
                          _radiusValue,
                        ), // 设置圆角半径为 8
                      ),
                    ),
                    onPressed: () {
                      widget.viewModel.update();
                    },
                    child: Text('开始转换'),
                  ),
                  _marginTop(40),
                  // 日志输出
                  _logText(),
                ],
              );
            },
          ),
        ),
      ),
    );
  }
}
