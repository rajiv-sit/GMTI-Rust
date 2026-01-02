#pragma once

#include <QVector>
#include <QWidget>

class StatusGraph : public QWidget
{
    Q_OBJECT

public:
    explicit StatusGraph(QWidget* parent = nullptr);

public slots:
    void updateData(const QVector<double>& profile, int detectionCount);

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    QVector<double> profile_;
    int detection_count_ = 0;
};
};
